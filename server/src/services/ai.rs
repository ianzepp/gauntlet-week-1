//! AI service — LLM prompt → tool calls → board mutations.
//!
//! DESIGN
//! ======
//! Receives an `ai:prompt` frame, sends the board state + user prompt to
//! the LLM with CollabBoard tools, executes returned tool calls as object
//! mutations, and broadcasts results to board peers.

use std::sync::Arc;

use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::frame::Data;
use crate::llm::LlmClient;
use crate::llm::tools::collaboard_tools;
use crate::llm::types::{Content, ContentBlock, Message};
use crate::state::{AppState, BoardObject};

const MAX_TOOL_ITERATIONS: usize = 10;
const MAX_TOKENS: u32 = 4096;

// =============================================================================
// TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("LLM not configured")]
    LlmNotConfigured,
    #[error("board not loaded: {0}")]
    BoardNotLoaded(Uuid),
    #[error("LLM error: {0}")]
    LlmError(#[from] crate::llm::types::LlmError),
    #[error("object error: {0}")]
    ObjectError(#[from] super::object::ObjectError),
}

impl crate::frame::ErrorCode for AiError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::LlmNotConfigured => "E_LLM_NOT_CONFIGURED",
            Self::BoardNotLoaded(_) => "E_BOARD_NOT_LOADED",
            Self::LlmError(_) => "E_LLM_ERROR",
            Self::ObjectError(_) => "E_OBJECT_ERROR",
        }
    }

    fn retryable(&self) -> bool {
        matches!(self, Self::LlmError(e) if e.retryable())
    }
}

/// Result of an AI prompt: mutated objects + optional text response.
pub struct AiResult {
    pub mutations: Vec<AiMutation>,
    pub text: Option<String>,
}

pub enum AiMutation {
    Created(BoardObject),
    Updated(BoardObject),
    Deleted(Uuid),
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================

pub async fn handle_prompt(
    state: &AppState,
    llm: &Arc<LlmClient>,
    board_id: Uuid,
    prompt: &str,
) -> Result<AiResult, AiError> {
    // Snapshot board objects for context.
    let board_snapshot = {
        let boards = state.boards.read().await;
        let board = boards.get(&board_id).ok_or(AiError::BoardNotLoaded(board_id))?;
        board.objects.values().cloned().collect::<Vec<_>>()
    };

    let system = build_system_prompt(&board_snapshot);
    let tools = collaboard_tools();

    let mut messages = vec![Message {
        role: "user".into(),
        content: Content::Text(prompt.into()),
    }];

    let mut all_mutations = Vec::new();
    let mut final_text: Option<String> = None;

    for iteration in 0..MAX_TOOL_ITERATIONS {
        let response = llm.chat(MAX_TOKENS, &system, &messages, Some(&tools)).await?;

        info!(
            iteration,
            stop_reason = %response.stop_reason,
            input_tokens = response.input_tokens,
            output_tokens = response.output_tokens,
            "ai: LLM response"
        );

        // Collect text blocks.
        let text_parts: Vec<&str> = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if !text_parts.is_empty() {
            final_text = Some(text_parts.join("\n"));
        }

        // Collect tool_use blocks.
        let tool_calls: Vec<(String, String, serde_json::Value)> = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
            .collect();

        // If no tool calls, we're done.
        if tool_calls.is_empty() {
            break;
        }

        // Push assistant message with the full content blocks.
        messages.push(Message {
            role: "assistant".into(),
            content: Content::Blocks(response.content),
        });

        // Execute each tool call and collect results.
        let mut tool_results = Vec::new();
        for (tool_id, tool_name, input) in &tool_calls {
            let result = execute_tool(state, board_id, tool_name, input, &mut all_mutations).await;
            let (content, is_error) = match result {
                Ok(msg) => (msg, None),
                Err(e) => (e.to_string(), Some(true)),
            };
            tool_results.push(ContentBlock::ToolResult {
                tool_use_id: tool_id.clone(),
                content,
                is_error,
            });
        }

        // Push tool results as a user message.
        messages.push(Message {
            role: "user".into(),
            content: Content::Blocks(tool_results),
        });

        // If stop_reason is not tool_use, break.
        if response.stop_reason != "tool_use" {
            break;
        }
    }

    Ok(AiResult { mutations: all_mutations, text: final_text })
}

// =============================================================================
// SYSTEM PROMPT
// =============================================================================

fn build_system_prompt(objects: &[BoardObject]) -> String {
    let mut prompt = String::from(
        "You are an AI assistant for CollabBoard, a collaborative whiteboard application. \
         You can create, move, update, and delete objects on the board using the provided tools.\n\n\
         Current board objects:\n",
    );

    if objects.is_empty() {
        prompt.push_str("(empty board — no objects yet)\n");
    } else {
        for obj in objects {
            let text = obj.props.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let color = obj.props.get("color").and_then(|v| v.as_str()).unwrap_or("");
            prompt.push_str(&format!(
                "- id={} kind={} x={:.0} y={:.0} text={:?} color={:?}\n",
                obj.id, obj.kind, obj.x, obj.y, text, color,
            ));
        }
    }

    prompt.push_str(
        "\nUse tools to manipulate the board. Place new objects with reasonable spacing \
         (e.g. 200px apart). Use varied colors for visual distinction.",
    );
    prompt
}

// =============================================================================
// TOOL EXECUTION
// =============================================================================

async fn execute_tool(
    state: &AppState,
    board_id: Uuid,
    tool_name: &str,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    match tool_name {
        "create_objects" => execute_create_objects(state, board_id, input, mutations).await,
        "move_objects" => execute_move_objects(state, board_id, input, mutations).await,
        "update_objects" => execute_update_objects(state, board_id, input, mutations).await,
        "delete_objects" => execute_delete_objects(state, board_id, input, mutations).await,
        "organize_layout" => execute_organize_layout(state, board_id, input, mutations).await,
        "summarize_board" => execute_summarize_board(state, board_id, input, mutations).await,
        "group_by_theme" => {
            // group_by_theme is an LLM-level operation; just acknowledge it.
            Ok("group_by_theme: acknowledged. Use update_objects to apply color changes.".into())
        }
        _ => Ok(format!("unknown tool: {tool_name}")),
    }
}

async fn execute_create_objects(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let objects = input.get("objects").and_then(|v| v.as_array());
    let Some(objects) = objects else {
        return Ok("error: missing 'objects' array".into());
    };

    let mut created_ids = Vec::new();
    for obj_def in objects {
        let kind = obj_def.get("kind").and_then(|v| v.as_str()).unwrap_or("sticky_note");
        let x = obj_def.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = obj_def.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let props = obj_def.get("props").cloned().unwrap_or(json!({}));

        let obj = super::object::create_object(state, board_id, kind, x, y, props, None).await?;
        created_ids.push(obj.id.to_string());
        mutations.push(AiMutation::Created(obj));
    }

    Ok(format!("created {} objects: [{}]", created_ids.len(), created_ids.join(", ")))
}

async fn execute_move_objects(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let moves = input.get("moves").and_then(|v| v.as_array());
    let Some(moves) = moves else {
        return Ok("error: missing 'moves' array".into());
    };

    let mut moved = 0;
    for mv in moves {
        let Some(id) = mv.get("id").and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok()) else {
            continue;
        };
        let mut data = Data::new();
        if let Some(x) = mv.get("x") {
            data.insert("x".into(), x.clone());
        }
        if let Some(y) = mv.get("y") {
            data.insert("y".into(), y.clone());
        }

        match super::object::update_object(state, board_id, id, &data, 0).await {
            Ok(obj) => {
                mutations.push(AiMutation::Updated(obj));
                moved += 1;
            }
            Err(e) => warn!(error = %e, %id, "ai: move_objects failed for object"),
        }
    }

    Ok(format!("moved {moved} objects"))
}

async fn execute_update_objects(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let updates = input.get("updates").and_then(|v| v.as_array());
    let Some(updates) = updates else {
        return Ok("error: missing 'updates' array".into());
    };

    let mut updated = 0;
    for upd in updates {
        let Some(id) = upd.get("id").and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok()) else {
            continue;
        };
        let mut data = Data::new();
        if let Some(props) = upd.get("props") {
            data.insert("props".into(), props.clone());
        }
        if let Some(w) = upd.get("width") {
            data.insert("width".into(), w.clone());
        }
        if let Some(h) = upd.get("height") {
            data.insert("height".into(), h.clone());
        }

        match super::object::update_object(state, board_id, id, &data, 0).await {
            Ok(obj) => {
                mutations.push(AiMutation::Updated(obj));
                updated += 1;
            }
            Err(e) => warn!(error = %e, %id, "ai: update_objects failed for object"),
        }
    }

    Ok(format!("updated {updated} objects"))
}

async fn execute_delete_objects(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let ids = input.get("ids").and_then(|v| v.as_array());
    let Some(ids) = ids else {
        return Ok("error: missing 'ids' array".into());
    };

    let mut deleted = 0;
    for id_val in ids {
        let Some(id) = id_val.as_str().and_then(|s| s.parse::<Uuid>().ok()) else {
            continue;
        };

        match super::object::delete_object(state, board_id, id).await {
            Ok(()) => {
                mutations.push(AiMutation::Deleted(id));
                deleted += 1;
            }
            Err(e) => warn!(error = %e, %id, "ai: delete_objects failed for object"),
        }
    }

    Ok(format!("deleted {deleted} objects"))
}

async fn execute_organize_layout(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let layout = input.get("layout").and_then(|v| v.as_str()).unwrap_or("grid");
    let spacing = input.get("spacing").and_then(|v| v.as_f64()).unwrap_or(200.0);

    // Get target IDs or all objects.
    let target_ids: Vec<Uuid> = if let Some(ids) = input.get("ids").and_then(|v| v.as_array()) {
        ids.iter()
            .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
            .collect()
    } else {
        let boards = state.boards.read().await;
        let Some(board) = boards.get(&board_id) else {
            return Ok("error: board not loaded".into());
        };
        board.objects.keys().copied().collect()
    };

    if target_ids.is_empty() {
        return Ok("no objects to organize".into());
    }

    let cols = (target_ids.len() as f64).sqrt().ceil() as usize;
    let mut moved = 0;

    for (i, id) in target_ids.iter().enumerate() {
        let (x, y) = match layout {
            "grid" => {
                let col = i % cols;
                let row = i / cols;
                (col as f64 * spacing + 100.0, row as f64 * spacing + 100.0)
            }
            "circle" => {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (target_ids.len() as f64);
                let radius = spacing * (target_ids.len() as f64).max(3.0) / (2.0 * std::f64::consts::PI);
                (500.0 + radius * angle.cos(), 500.0 + radius * angle.sin())
            }
            _ => {
                // cluster / tree fallback to grid
                let col = i % cols;
                let row = i / cols;
                (col as f64 * spacing + 100.0, row as f64 * spacing + 100.0)
            }
        };

        let mut data = Data::new();
        data.insert("x".into(), json!(x));
        data.insert("y".into(), json!(y));

        match super::object::update_object(state, board_id, *id, &data, 0).await {
            Ok(obj) => {
                mutations.push(AiMutation::Updated(obj));
                moved += 1;
            }
            Err(e) => warn!(error = %e, %id, "ai: organize_layout failed for object"),
        }
    }

    Ok(format!("organized {moved} objects in {layout} layout"))
}

async fn execute_summarize_board(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    // Collect all text from board objects.
    let texts: Vec<String> = {
        let boards = state.boards.read().await;
        let Some(board) = boards.get(&board_id) else {
            return Ok("error: board not loaded".into());
        };
        board
            .objects
            .values()
            .filter_map(|obj| obj.props.get("text").and_then(|v| v.as_str()).map(String::from))
            .collect()
    };

    let summary = if texts.is_empty() {
        "No text content on the board.".to_string()
    } else {
        format!("Board contains {} items: {}", texts.len(), texts.join("; "))
    };

    let x = input
        .get("position")
        .and_then(|p| p.get("x"))
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0);
    let y = input
        .get("position")
        .and_then(|p| p.get("y"))
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0);

    let obj = super::object::create_object(
        state,
        board_id,
        "sticky_note",
        x,
        y,
        json!({"text": summary, "color": "#FFE066"}),
        None,
    )
    .await?;

    let result = format!("created summary note: {}", obj.id);
    mutations.push(AiMutation::Created(obj));

    Ok(result)
}
