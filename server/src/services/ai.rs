//! AI service — LLM prompt → tool calls → board mutations.
//!
//! DESIGN
//! ======
//! Receives an `ai:prompt` frame, sends the board state + user prompt to
//! the LLM with `CollabBoard` tools, executes returned tool calls as object
//! mutations, and broadcasts results to board peers.
//!
//! Tool names match the G4 Week 1 spec exactly (issue #19):
//! createStickyNote, createShape, createFrame, createConnector,
//! moveObject, resizeObject, updateText, changeColor, getBoardState.

use std::fmt::Write;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::frame::{Data, Frame};
use crate::llm::LlmChat;
use crate::llm::tools::gauntlet_week_1_tools;
use crate::llm::types::{Content, ContentBlock, Message};
use crate::state::{AppState, BoardObject};

const DEFAULT_AI_MAX_TOOL_ITERATIONS: usize = 10;
const DEFAULT_AI_MAX_TOKENS: u32 = 4096;
const MAX_SESSION_CONVERSATION_MESSAGES: usize = 20;
const BASE_SYSTEM_PROMPT: &str = include_str!("../llm/system.md");

fn env_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

fn ai_max_tool_iterations() -> usize {
    static VALUE: OnceLock<usize> = OnceLock::new();
    *VALUE.get_or_init(|| env_parse("AI_MAX_TOOL_ITERATIONS", DEFAULT_AI_MAX_TOOL_ITERATIONS))
}

fn ai_max_tokens() -> u32 {
    static VALUE: OnceLock<u32> = OnceLock::new();
    *VALUE.get_or_init(|| env_parse("AI_MAX_TOKENS", DEFAULT_AI_MAX_TOKENS))
}

fn trace_id_for_prompt(parent_frame_id: Option<Uuid>) -> Uuid {
    parent_frame_id.unwrap_or_else(Uuid::new_v4)
}

fn trace_meta(
    trace_id: Uuid,
    span_id: Uuid,
    parent_span_id: Option<Uuid>,
    kind: &str,
    label: Option<&str>,
) -> serde_json::Value {
    let mut trace = serde_json::Map::new();
    trace.insert("trace_id".into(), json!(trace_id));
    trace.insert("span_id".into(), json!(span_id));
    trace.insert("parent_span_id".into(), json!(parent_span_id));
    trace.insert("kind".into(), json!(kind));
    if let Some(label) = label {
        trace.insert("label".into(), json!(label));
    }
    serde_json::Value::Object(trace)
}

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
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("invalid tool syscall: {0}")]
    InvalidToolSyscall(String),
}

impl crate::frame::ErrorCode for AiError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::LlmNotConfigured => "E_LLM_NOT_CONFIGURED",
            Self::BoardNotLoaded(_) => "E_BOARD_NOT_LOADED",
            Self::LlmError(_) => "E_LLM_ERROR",
            Self::ObjectError(_) => "E_OBJECT_ERROR",
            Self::RateLimited(_) => "E_RATE_LIMITED",
            Self::InvalidToolSyscall(_) => "E_INVALID_TOOL_SYSCALL",
        }
    }

    fn retryable(&self) -> bool {
        matches!(self, Self::LlmError(e) if e.retryable()) || matches!(self, Self::RateLimited(_))
    }
}

impl From<crate::rate_limit::RateLimitError> for AiError {
    fn from(e: crate::rate_limit::RateLimitError) -> Self {
        Self::RateLimited(e.to_string())
    }
}

/// Result of an AI prompt: mutated objects + optional text response.
#[derive(Debug)]
pub struct AiResult {
    pub mutations: Vec<AiMutation>,
    pub text: Option<String>,
}

#[derive(Debug)]
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
    llm: &Arc<dyn LlmChat>,
    board_id: Uuid,
    client_id: Uuid,
    user_id: Uuid,
    prompt: &str,
    grid_context: Option<&str>,
) -> Result<AiResult, AiError> {
    handle_prompt_with_parent(state, llm, board_id, client_id, user_id, prompt, grid_context, None).await
}

pub async fn handle_prompt_with_parent(
    state: &AppState,
    llm: &Arc<dyn LlmChat>,
    board_id: Uuid,
    client_id: Uuid,
    user_id: Uuid,
    prompt: &str,
    grid_context: Option<&str>,
    parent_frame_id: Option<Uuid>,
) -> Result<AiResult, AiError> {
    info!(%board_id, %client_id, prompt_len = prompt.len(), "ai: prompt received");
    let max_tool_iterations = ai_max_tool_iterations();
    let max_tokens = ai_max_tokens();

    // Rate-limit check: per-client + global request limits.
    state.rate_limiter.check_and_record(client_id)?;

    // Snapshot board objects for context.
    let board_snapshot = {
        let boards = state.boards.read().await;
        let board = boards
            .get(&board_id)
            .ok_or(AiError::BoardNotLoaded(board_id))?;
        board.objects.values().cloned().collect::<Vec<_>>()
    };

    let system = build_system_prompt(&board_snapshot, grid_context);
    let tools = gauntlet_week_1_tools();
    let session_key = (client_id, board_id);
    let prior_session_messages = load_session_messages(state, session_key).await;

    // Keep persisted context scoped to the active websocket session.
    // Refreshing reconnects and clears this memory.
    let prompt_message =
        Message { role: "user".into(), content: Content::Text(format!("<user_input>{prompt}</user_input>")) };
    let mut base_messages = prior_session_messages;
    base_messages.push(prompt_message.clone());
    let mut latest_tool_exchange: Option<(Message, Message)> = None;

    let mut all_mutations = Vec::new();
    let mut final_text: Option<String> = None;
    let token_reservation = u64::from(max_tokens);
    let trace_id = trace_id_for_prompt(parent_frame_id);

    for iteration in 0..max_tool_iterations {
        let mut llm_req = Frame::request("ai:llm_request", Data::new())
            .with_board_id(board_id)
            .with_from(user_id.to_string());
        llm_req.parent_id = parent_frame_id;
        let mut llm_req_trace = trace_meta(trace_id, llm_req.id, parent_frame_id, "ai.llm_request", Some("llm"))
            .as_object()
            .cloned()
            .unwrap_or_default();
        llm_req_trace.insert("iteration".into(), json!(iteration));
        llm_req
            .data
            .insert("trace".into(), serde_json::Value::Object(llm_req_trace));
        super::persistence::enqueue_frame(state, &llm_req);

        let mut llm_messages = base_messages.clone();
        if let Some((assistant_tools, tool_results)) = latest_tool_exchange.clone() {
            llm_messages.push(assistant_tools);
            llm_messages.push(tool_results);
        }
        let llm_started_at = Instant::now();
        state
            .rate_limiter
            .reserve_token_budget(client_id, token_reservation)?;
        let response = match llm
            .chat(max_tokens, &system, &llm_messages, Some(&tools))
            .await
        {
            Ok(response) => response,
            Err(err) => {
                let duration_ms = i64::try_from(llm_started_at.elapsed().as_millis()).unwrap_or(i64::MAX);
                let mut err_frame = llm_req.error_from(&err);
                let mut err_trace = trace_meta(trace_id, llm_req.id, parent_frame_id, "ai.llm_request", Some("llm"))
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                err_trace.insert("iteration".into(), json!(iteration));
                err_trace.insert("duration_ms".into(), json!(duration_ms));
                err_frame
                    .data
                    .insert("trace".into(), serde_json::Value::Object(err_trace));
                super::persistence::enqueue_frame(state, &err_frame);
                state
                    .rate_limiter
                    .release_reserved_tokens(client_id, token_reservation);
                return Err(err.into());
            }
        };
        let duration_ms = i64::try_from(llm_started_at.elapsed().as_millis()).unwrap_or(i64::MAX);
        let total_tokens = response.input_tokens + response.output_tokens;

        let mut llm_done_data = Data::new();
        let mut llm_trace = trace_meta(trace_id, llm_req.id, parent_frame_id, "ai.llm_request", Some(&response.model))
            .as_object()
            .cloned()
            .unwrap_or_default();
        llm_trace.insert("iteration".into(), json!(iteration));
        llm_trace.insert("input_tokens".into(), json!(response.input_tokens));
        llm_trace.insert("output_tokens".into(), json!(response.output_tokens));
        llm_trace.insert("tokens".into(), json!(total_tokens));
        llm_trace.insert("duration_ms".into(), json!(duration_ms));
        llm_trace.insert("stop_reason".into(), json!(response.stop_reason.clone()));
        llm_done_data.insert("trace".into(), serde_json::Value::Object(llm_trace));
        let llm_done = llm_req.done_with(llm_done_data);
        super::persistence::enqueue_frame(state, &llm_done);

        info!(
            iteration,
            stop_reason = %response.stop_reason,
            input_tokens = response.input_tokens,
            output_tokens = response.output_tokens,
            "ai: LLM response"
        );

        // Record token usage for budget tracking.
        state
            .rate_limiter
            .record_tokens(client_id, total_tokens, token_reservation);

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
                ContentBlock::ToolUse { id, name, input } => Some((id.clone(), name.clone(), input.clone())),
                _ => None,
            })
            .collect();

        // If no tool calls, we're done.
        if tool_calls.is_empty() {
            break;
        }

        // Execute each tool call and collect results.
        let mut tool_results = Vec::new();
        for (tool_id, tool_name, input) in &tool_calls {
            info!(iteration, tool = %tool_name, "ai: executing tool via syscall");
            let result = execute_tool_via_syscall(
                state,
                board_id,
                user_id,
                iteration,
                tool_id,
                tool_name,
                input,
                trace_id,
                Some(llm_req.id),
                &mut all_mutations,
            )
            .await;
            let (content, is_error) = match &result {
                Ok(msg) => {
                    info!(iteration, tool = %tool_name, "ai: tool ok — {msg}");
                    (msg.clone(), None)
                }
                Err(e) => {
                    warn!(iteration, tool = %tool_name, error = %e, "ai: tool error");
                    (e.to_string(), Some(true))
                }
            };
            tool_results.push(ContentBlock::ToolResult { tool_use_id: tool_id.clone(), content, is_error });
        }

        // Only carry the most recent tool-call exchange forward between tool rounds.
        latest_tool_exchange = Some((
            Message { role: "assistant".into(), content: Content::Blocks(response.content) },
            Message { role: "user".into(), content: Content::Blocks(tool_results) },
        ));

        // If stop_reason is not tool_use, break.
        if response.stop_reason != "tool_use" {
            break;
        }
    }

    // Guarantee the client always receives a response payload by synthesizing
    // fallback text when the LLM returned none (e.g. thinking-only or
    // mutations-only responses).
    if final_text.is_none() {
        final_text = Some(if all_mutations.is_empty() {
            "Done.".into()
        } else {
            format!("Done — {} object(s) updated.", all_mutations.len())
        });
    }

    info!(
        %board_id,
        mutations = all_mutations.len(),
        has_text = final_text.is_some(),
        "ai: prompt complete"
    );

    if let Some(text) = final_text.clone() {
        append_session_messages(state, session_key, prompt_message, text).await;
    }

    Ok(AiResult { mutations: all_mutations, text: final_text })
}

async fn execute_tool_via_syscall(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
    iteration: usize,
    tool_use_id: &str,
    tool_name: &str,
    input: &serde_json::Value,
    trace_id: Uuid,
    parent_frame_id: Option<Uuid>,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let syscall = format!("tool:{tool_name}");
    let mut req_data = Data::new();
    req_data.insert("tool_use_id".into(), json!(tool_use_id));
    req_data.insert("input".into(), input.clone());

    let mut req = Frame::request(syscall, req_data)
        .with_board_id(board_id)
        .with_from(user_id.to_string());
    req.parent_id = parent_frame_id;
    let mut req_trace = trace_meta(trace_id, req.id, parent_frame_id, "ai.tool_call", Some(tool_name))
        .as_object()
        .cloned()
        .unwrap_or_default();
    req_trace.insert("iteration".into(), json!(iteration));
    req.data
        .insert("trace".into(), serde_json::Value::Object(req_trace));
    super::persistence::enqueue_frame(state, &req);

    match super::tool_syscall::dispatch_tool_frame(state, board_id, &req).await {
        Ok(outcome) => {
            mutations.extend(outcome.mutations);
            let mut done_data = outcome.done_data;
            done_data.insert(
                "trace".into(),
                trace_meta(trace_id, req.id, parent_frame_id, "ai.tool_call", Some(tool_name)),
            );
            let done = req.done_with(done_data);
            super::persistence::enqueue_frame(state, &done);
            Ok(outcome.content)
        }
        Err(err) => {
            let mut error = req.error_from(&err);
            error.data.insert("tool_use_id".into(), json!(tool_use_id));
            error.data.insert(
                "trace".into(),
                trace_meta(trace_id, req.id, parent_frame_id, "ai.tool_call", Some(tool_name)),
            );
            super::persistence::enqueue_frame(state, &error);
            Err(err)
        }
    }
}

async fn load_session_messages(state: &AppState, session_key: (Uuid, Uuid)) -> Vec<Message> {
    state
        .ai_session_messages
        .read()
        .await
        .get(&session_key)
        .cloned()
        .unwrap_or_default()
}

async fn append_session_messages(state: &AppState, session_key: (Uuid, Uuid), user: Message, assistant_text: String) {
    let mut sessions = state.ai_session_messages.write().await;
    let entry = sessions.entry(session_key).or_default();
    entry.push(user);
    entry.push(Message { role: "assistant".into(), content: Content::Text(assistant_text) });
    if entry.len() > MAX_SESSION_CONVERSATION_MESSAGES {
        // Round up to even so we never split a user/assistant pair —
        // the Anthropic API requires messages to start with a user role.
        let extra = entry.len() - MAX_SESSION_CONVERSATION_MESSAGES;
        let extra = extra + (extra % 2);
        entry.drain(0..extra.min(entry.len()));
    }
}

// =============================================================================
// SYSTEM PROMPT
// =============================================================================

pub(crate) fn build_system_prompt(objects: &[BoardObject], grid_context: Option<&str>) -> String {
    let mut prompt = String::from(BASE_SYSTEM_PROMPT.trim_end());
    prompt.push_str("\n\nCurrent board objects:\n");

    if objects.is_empty() {
        prompt.push_str("(empty board — no objects yet)\n");
    } else {
        for obj in objects {
            let text = obj.props.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let title = obj
                .props
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let label = if !text.is_empty() {
                text
            } else if !title.is_empty() {
                title
            } else {
                ""
            };
            let props_json =
                serde_json::to_string(&obj.props).unwrap_or_else(|_| "{\"error\":\"props_serialize\"}".to_owned());
            let _ = writeln!(
                prompt,
                "- id={} kind={} x={:.0} y={:.0} w={} h={} label={:?} props={}",
                obj.id,
                obj.kind,
                obj.x,
                obj.y,
                obj.width.map_or("-".into(), |w| format!("{w:.0}")),
                obj.height.map_or("-".into(), |h| format!("{h:.0}")),
                label,
                props_json,
            );
        }
    }

    if let Some(grid) = grid_context {
        prompt.push('\n');
        prompt.push_str(grid);
        prompt.push('\n');
    }
    prompt
}

// =============================================================================
// TOOL EXECUTION
// =============================================================================

pub(crate) async fn execute_tool(
    state: &AppState,
    board_id: Uuid,
    tool_name: &str,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    match tool_name {
        "createStickyNote" => execute_create_sticky_note(state, board_id, input, mutations).await,
        "createShape" => execute_create_shape(state, board_id, input, mutations).await,
        "createFrame" => execute_create_frame(state, board_id, input, mutations).await,
        "createConnector" => execute_create_connector(state, board_id, input, mutations).await,
        "rotateObject" => execute_rotate_object(state, board_id, input, mutations).await,
        "moveObject" => execute_move_object(state, board_id, input, mutations).await,
        "resizeObject" => execute_resize_object(state, board_id, input, mutations).await,
        "updateText" => execute_update_text(state, board_id, input, mutations).await,
        "updateTextStyle" => execute_update_text_style(state, board_id, input, mutations).await,
        "changeColor" => execute_change_color(state, board_id, input, mutations).await,
        "getBoardState" => execute_get_board_state(state, board_id).await,
        _ => Ok(format!("unknown tool: {tool_name}")),
    }
}

async fn get_object_snapshot(
    state: &AppState,
    board_id: Uuid,
    object_id: Uuid,
) -> Result<BoardObject, super::object::ObjectError> {
    let boards = state.boards.read().await;
    let board = boards
        .get(&board_id)
        .ok_or(super::object::ObjectError::BoardNotLoaded(board_id))?;
    let obj = board
        .objects
        .get(&object_id)
        .ok_or(super::object::ObjectError::NotFound(object_id))?;
    Ok(obj.clone())
}

async fn update_object_with_retry<F>(
    state: &AppState,
    board_id: Uuid,
    object_id: Uuid,
    build_updates: F,
) -> Result<BoardObject, super::object::ObjectError>
where
    F: Fn(&BoardObject) -> Data,
{
    for attempt in 0..2 {
        let snapshot = get_object_snapshot(state, board_id, object_id).await?;
        let updates = build_updates(&snapshot);
        match super::object::update_object(state, board_id, object_id, &updates, snapshot.version).await {
            Ok(obj) => return Ok(obj),
            Err(super::object::ObjectError::StaleUpdate { .. }) if attempt == 0 => {
                // Retry once with a fresh snapshot in case another update won the race.
            }
            Err(e) => return Err(e),
        }
    }

    // Loop always returns on success or terminal error.
    Err(super::object::ObjectError::NotFound(object_id))
}

async fn execute_create_sticky_note(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let text = input.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let fill = input
        .get("fill")
        .and_then(|v| v.as_str())
        .unwrap_or("#FFEB3B");
    let stroke = input.get("stroke").and_then(|v| v.as_str()).unwrap_or(fill);
    let stroke_width = input
        .get("strokeWidth")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let font_size = input
        .get("fontSize")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(24.0);
    let text_color = input
        .get("textColor")
        .and_then(|v| v.as_str())
        .unwrap_or("#1F1A17");

    let props = json!({
        "title": title,
        "text": text,
        "fontSize": font_size,
        "textColor": text_color,
        "fill": fill,
        "stroke": stroke,
        "strokeWidth": stroke_width
    });
    let obj =
        super::object::create_object(state, board_id, "sticky_note", x, y, None, None, 0.0, props, None, None).await?;
    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("created sticky note {id}"))
}

async fn execute_create_shape(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let requested_kind = input
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("rectangle");
    let Some(kind) = canonical_kind(requested_kind) else {
        return Ok("error: unsupported shape type".into());
    };
    let x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let fill = input
        .get("fill")
        .and_then(|v| v.as_str())
        .unwrap_or("#4CAF50")
        .to_string();
    let stroke = input
        .get("stroke")
        .and_then(|v| v.as_str())
        .map_or_else(|| fill.clone(), ToOwned::to_owned);
    let stroke_width = input
        .get("strokeWidth")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    let props = if kind == "text" {
        let text_color = input
            .get("textColor")
            .and_then(|v| v.as_str())
            .unwrap_or("#1F1A17");
        json!({
            "text": input.get("text").and_then(|v| v.as_str()).unwrap_or("Text"),
            "fontSize": input.get("fontSize").and_then(serde_json::Value::as_f64).unwrap_or(24.0),
            "textColor": text_color
        })
    } else if kind == "line" || kind == "arrow" {
        let width = input
            .get("width")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(180.0);
        let height = input
            .get("height")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let a = json!({ "x": x, "y": y });
        let b = json!({ "x": x + width, "y": y + height });
        json!({
            "a": a,
            "b": b,
            "stroke": stroke,
            "strokeWidth": stroke_width
        })
    } else {
        json!({
            "fill": fill,
            "stroke": stroke,
            "strokeWidth": stroke_width
        })
    };
    let w = input.get("width").and_then(serde_json::Value::as_f64);
    let h = input.get("height").and_then(serde_json::Value::as_f64);
    let default_w = if kind == "text" {
        220.0
    } else if kind == "line" || kind == "arrow" {
        180.0
    } else {
        160.0
    };
    let default_h = if kind == "text" {
        56.0
    } else if kind == "line" || kind == "arrow" {
        0.0
    } else {
        100.0
    };
    let mut obj = super::object::create_object(state, board_id, &kind, x, y, w, h, 0.0, props, None, None).await?;

    // Update the in-memory object with dimensions.
    if obj.width.is_some() || obj.height.is_some() || kind == "text" {
        let mut data = Data::new();
        if let Some(w) = obj.width.or(Some(default_w)) {
            data.insert("width".into(), json!(w));
        }
        if let Some(h) = obj.height.or(Some(default_h)) {
            data.insert("height".into(), json!(h));
        }
        obj = super::object::update_object(state, board_id, obj.id, &data, obj.version).await?;
    }

    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("created {requested_kind} shape {id}"))
}

async fn execute_create_frame(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let title = input
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled");
    let x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let stroke = input
        .get("stroke")
        .and_then(|v| v.as_str())
        .unwrap_or("#1F1A17");
    let stroke_width = input
        .get("strokeWidth")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    let props = json!({
        "title": title,
        "stroke": stroke,
        "strokeWidth": stroke_width
    });
    let w = input
        .get("width")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(400.0);
    let h = input
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(300.0);
    let obj =
        super::object::create_object(state, board_id, "frame", x, y, Some(w), Some(h), 0.0, props, None, None).await?;
    let obj_id = obj.id;
    let mut data = Data::new();
    data.insert("width".into(), json!(w));
    data.insert("height".into(), json!(h));
    let obj = super::object::update_object(state, board_id, obj_id, &data, obj.version).await?;

    mutations.push(AiMutation::Created(obj));
    Ok(format!("created frame \"{title}\" {obj_id}"))
}

async fn execute_create_connector(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let from_id_raw = input.get("fromId").and_then(|v| v.as_str()).unwrap_or("");
    let to_id_raw = input.get("toId").and_then(|v| v.as_str()).unwrap_or("");
    let style = input
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("arrow");
    let from_id = match from_id_raw.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Ok("error: missing or invalid fromId".into()),
    };
    let to_id = match to_id_raw.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Ok("error: missing or invalid toId".into()),
    };

    let kind = if style.eq_ignore_ascii_case("line") {
        "line"
    } else {
        "arrow"
    };
    let (ax, ay) = object_center(state, board_id, from_id).await?;
    let (bx, by) = object_center(state, board_id, to_id).await?;
    let mut props = serde_json::Map::new();
    props.insert(
        "a".into(),
        json!({
            "type": "attached",
            "object_id": from_id,
            "ux": 0.5,
            "uy": 0.5,
            "x": ax,
            "y": ay
        }),
    );
    props.insert(
        "b".into(),
        json!({
            "type": "attached",
            "object_id": to_id,
            "ux": 0.5,
            "uy": 0.5,
            "x": bx,
            "y": by
        }),
    );
    props.insert("style".into(), json!(style));
    props.insert("stroke".into(), json!("#D94B4B"));
    props.insert("strokeWidth".into(), json!(2.0));

    let width = (bx - ax).abs().max(1.0);
    let height = (by - ay).abs();
    let obj = super::object::create_object(
        state,
        board_id,
        kind,
        ax.min(bx),
        ay.min(by),
        Some(width),
        Some(height),
        0.0,
        serde_json::Value::Object(props),
        None,
        None,
    )
    .await?;
    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("created connector {id} from {from_id} to {to_id}"))
}

async fn execute_rotate_object(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(id) = input
        .get("objectId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
    else {
        return Ok("error: missing or invalid objectId".into());
    };

    let Some(rotation) = input.get("rotation").and_then(serde_json::Value::as_f64) else {
        return Ok("error: missing rotation".into());
    };

    match update_object_with_retry(state, board_id, id, |_| {
        let mut data = Data::new();
        data.insert("rotation".into(), json!(rotation));
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("rotated object {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: rotateObject failed");
            Ok(format!("error rotating {id}: {e}"))
        }
    }
}

async fn execute_move_object(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(id) = input
        .get("objectId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
    else {
        return Ok("error: missing or invalid objectId".into());
    };

    let x = input.get("x").cloned();
    let y = input.get("y").cloned();

    match update_object_with_retry(state, board_id, id, |_| {
        let mut data = Data::new();
        if let Some(value) = &x {
            data.insert("x".into(), value.clone());
        }
        if let Some(value) = &y {
            data.insert("y".into(), value.clone());
        }
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("moved object {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: moveObject failed");
            Ok(format!("error moving {id}: {e}"))
        }
    }
}

async fn execute_resize_object(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(id) = input
        .get("objectId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
    else {
        return Ok("error: missing or invalid objectId".into());
    };

    let width = input.get("width").cloned();
    let height = input.get("height").cloned();

    match update_object_with_retry(state, board_id, id, |_| {
        let mut data = Data::new();
        if let Some(value) = &width {
            data.insert("width".into(), value.clone());
        }
        if let Some(value) = &height {
            data.insert("height".into(), value.clone());
        }
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("resized object {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: resizeObject failed");
            Ok(format!("error resizing {id}: {e}"))
        }
    }
}

async fn execute_update_text(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(id) = input
        .get("objectId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
    else {
        return Ok("error: missing or invalid objectId".into());
    };

    let new_text = input
        .get("newText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let field = input
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("text")
        .to_string();
    if !matches!(field.as_str(), "text" | "title") {
        return Ok("error: field must be one of text/title".into());
    }

    match update_object_with_retry(state, board_id, id, |snapshot| {
        let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
        props.insert(field.clone(), json!(new_text));
        let mut data = Data::new();
        data.insert("props".into(), json!(props));
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("updated text on {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: updateText failed");
            Ok(format!("error updating text on {id}: {e}"))
        }
    }
}

async fn execute_update_text_style(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(id) = input
        .get("objectId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
    else {
        return Ok("error: missing or invalid objectId".into());
    };

    let text_color = input
        .get("textColor")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let font_size = input.get("fontSize").and_then(serde_json::Value::as_f64);
    if text_color.is_none() && font_size.is_none() {
        return Ok("error: provide textColor and/or fontSize".into());
    }

    match update_object_with_retry(state, board_id, id, |snapshot| {
        let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
        if let Some(color) = &text_color {
            props.insert("textColor".into(), json!(color));
        }
        if let Some(size) = font_size {
            props.insert("fontSize".into(), json!(size));
        }
        let mut data = Data::new();
        data.insert("props".into(), json!(props));
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("updated text style on {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: updateTextStyle failed");
            Ok(format!("error updating text style on {id}: {e}"))
        }
    }
}

async fn execute_change_color(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(id) = input
        .get("objectId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Uuid>().ok())
    else {
        return Ok("error: missing or invalid objectId".into());
    };

    let fill = input
        .get("fill")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let stroke = input
        .get("stroke")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let stroke_width = input.get("strokeWidth").and_then(serde_json::Value::as_f64);
    let text_color = input
        .get("textColor")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);

    if fill.is_none() && stroke.is_none() && stroke_width.is_none() && text_color.is_none() {
        return Ok("error: provide one of fill/stroke/strokeWidth/textColor".into());
    }

    match update_object_with_retry(state, board_id, id, |snapshot| {
        let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
        if let Some(width) = stroke_width {
            props.insert("strokeWidth".into(), json!(width));
        }

        if let Some(next_fill) = fill.clone() {
            props.insert("fill".into(), json!(next_fill));
        }

        if let Some(next_stroke) = stroke.clone() {
            props.insert("stroke".into(), json!(next_stroke));
        }
        if let Some(color) = text_color.clone() {
            props.insert("textColor".into(), json!(color));
        }
        let mut data = Data::new();
        data.insert("props".into(), json!(props));
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("changed style of {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: changeColor failed");
            Ok(format!("error changing color on {id}: {e}"))
        }
    }
}

async fn execute_get_board_state(state: &AppState, board_id: Uuid) -> Result<String, AiError> {
    let boards = state.boards.read().await;
    let Some(board) = boards.get(&board_id) else {
        return Ok("error: board not loaded".into());
    };

    let objects: Vec<serde_json::Value> = board
        .objects
        .values()
        .map(|obj| {
            json!({
                "id": obj.id,
                "kind": obj.kind,
                "x": obj.x,
                "y": obj.y,
                "width": obj.width,
                "height": obj.height,
                "rotation": obj.rotation,
                "z_index": obj.z_index,
                "props": obj.props,
                "version": obj.version,
            })
        })
        .collect();

    Ok(json!({ "objects": objects, "count": objects.len() }).to_string())
}

fn canonical_kind(kind: &str) -> Option<String> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "rectangle" => Some("rectangle".to_owned()),
        "ellipse" => Some("ellipse".to_owned()),
        "text" => Some("text".to_owned()),
        "line" => Some("line".to_owned()),
        "arrow" => Some("arrow".to_owned()),
        _ => None,
    }
}

async fn object_center(state: &AppState, board_id: Uuid, object_id: Uuid) -> Result<(f64, f64), AiError> {
    let obj = get_object_snapshot(state, board_id, object_id).await?;
    let width = obj.width.unwrap_or(0.0).max(0.0);
    let height = obj.height.unwrap_or(0.0).max(0.0);
    Ok((obj.x + (width * 0.5), obj.y + (height * 0.5)))
}

#[cfg(test)]
#[path = "ai_test.rs"]
mod tests;
