//! Internal tool syscall dispatcher shared by AI and websocket handlers.
//!
//! Tool syscalls arrive as `Frame` messages with a `syscall` field of the form `"tool:<name>"`.
//! This module strips the prefix and routes the operation to the AI tool-execution layer,
//! then assembles the result into the `done` frame payload that is returned to the caller.

use serde_json::json;
use uuid::Uuid;

use crate::frame::{Data, Frame};
use crate::state::AppState;

use super::ai::{AiError, AiMutation};

/// The result of executing a single tool syscall.
///
/// Bundles together the human-readable output string, the structured `done`-frame payload
/// ready to send back to the client, and any board mutations that the tool applied so that
/// callers can broadcast them to other connected clients.
pub(crate) struct ToolSyscallResult {
    /// Human-readable result text returned by the tool, suitable for display or AI context.
    pub content: String,
    /// Structured data for the `done` response frame, including `tool_use_id`, `content`, and
    /// the count of mutations applied.
    pub done_data: Data,
    /// Board object mutations produced by the tool (creates, updates, deletes).
    pub mutations: Vec<AiMutation>,
}

/// Parse a `tool:<name>` syscall frame, execute the named tool, and return a [`ToolSyscallResult`].
///
/// The `syscall` field of `req` must have the form `"tool:<operation>"`. The operation name is
/// extracted, the `"input"` field of `req.data` is forwarded to the AI tool executor, and the
/// resulting content and mutations are packaged for the caller to broadcast and acknowledge.
///
/// # Errors
///
/// Returns [`AiError::InvalidToolSyscall`] when the syscall string is missing or empty after
/// stripping the `"tool:"` prefix. Propagates any error returned by the underlying tool executor.
pub(crate) async fn dispatch_tool_frame(
    state: &AppState,
    board_id: Uuid,
    req: &Frame,
) -> Result<ToolSyscallResult, AiError> {
    let tool_name = req.syscall.split_once(':').map_or("", |(_, op)| op).trim();
    if tool_name.is_empty() {
        return Err(AiError::InvalidToolSyscall(req.syscall.clone()));
    }

    let input = req
        .data
        .get("input")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    let mut mutations = Vec::new();
    let content = super::ai::execute_tool(state, board_id, tool_name, &input, &mut mutations).await?;

    let mut done_data = Data::new();
    if let Some(tool_use_id) = req.data.get("tool_use_id") {
        done_data.insert("tool_use_id".into(), tool_use_id.clone());
    }
    done_data.insert("content".into(), json!(content));
    done_data.insert("mutations".into(), json!(mutations.len()));

    Ok(ToolSyscallResult { content, done_data, mutations })
}
