//! Internal tool syscall dispatcher shared by AI and websocket handlers.

use serde_json::json;
use uuid::Uuid;

use crate::frame::{Data, Frame};
use crate::state::AppState;

use super::ai::{AiError, AiMutation};

pub(crate) struct ToolSyscallResult {
    pub content: String,
    pub done_data: Data,
    pub mutations: Vec<AiMutation>,
}

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
