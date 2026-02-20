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

use futures::future::join_all;
use serde::Deserialize;
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
const MAX_YAML_CHANGE_OPS: usize = 500;
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

#[derive(Debug, Deserialize)]
struct YamlChangeDocument {
    changes: YamlChanges,
}

#[derive(Debug, Deserialize, Default)]
struct YamlChanges {
    #[serde(default)]
    create: Vec<YamlCreateChange>,
    #[serde(default)]
    update: Vec<YamlUpdateChange>,
    #[serde(default)]
    delete: Vec<YamlDeleteChange>,
}

#[derive(Debug, Deserialize)]
struct YamlCreateChange {
    kind: String,
    x: serde_yaml::Value,
    y: serde_yaml::Value,
    width: Option<serde_yaml::Value>,
    height: Option<serde_yaml::Value>,
    rotation: Option<serde_yaml::Value>,
    z: Option<serde_yaml::Value>,
    #[serde(default)]
    props: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct YamlUpdateChange {
    id: String,
    x: Option<serde_yaml::Value>,
    y: Option<serde_yaml::Value>,
    width: Option<serde_yaml::Value>,
    height: Option<serde_yaml::Value>,
    rotation: Option<serde_yaml::Value>,
    z: Option<serde_yaml::Value>,
    #[serde(default)]
    props: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct YamlDeleteChange {
    id: String,
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
        state.rate_limiter.record_tokens(
            client_id,
            total_tokens,
            token_reservation,
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
        let extra = entry.len() - MAX_SESSION_CONVERSATION_MESSAGES;
        entry.drain(0..extra);
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
        "batch" => execute_batch(state, board_id, input, mutations).await,
        "createStickyNote" => execute_create_sticky_note(state, board_id, input, mutations).await,
        "createShape" => execute_create_shape(state, board_id, input, mutations).await,
        "createFrame" => execute_create_frame(state, board_id, input, mutations).await,
        "createConnector" => execute_create_connector(state, board_id, input, mutations).await,
        "moveObject" => execute_move_object(state, board_id, input, mutations).await,
        "resizeObject" => execute_resize_object(state, board_id, input, mutations).await,
        "updateText" => execute_update_text(state, board_id, input, mutations).await,
        "changeColor" => execute_change_color(state, board_id, input, mutations).await,
        "getBoardState" => execute_get_board_state(state, board_id).await,
        "applyChangesYaml" => execute_apply_changes_yaml(state, board_id, input, mutations).await,
        _ => Ok(format!("unknown tool: {tool_name}")),
    }
}

async fn execute_batch(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(calls) = input.get("calls").and_then(serde_json::Value::as_array) else {
        return Ok("error: missing calls array".into());
    };

    let tasks = calls.iter().enumerate().map(|(index, call)| {
        let tool = call
            .get("tool")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        let call_input = call.get("input").cloned().unwrap_or_else(|| json!({}));

        async move {
            if tool.is_empty() {
                return (
                    json!({
                        "index": index,
                        "tool": "",
                        "ok": false,
                        "result": "error: missing tool"
                    }),
                    Vec::new(),
                );
            }
            if tool == "batch" {
                return (
                    json!({
                        "index": index,
                        "tool": tool,
                        "ok": false,
                        "result": "error: nested batch is not allowed"
                    }),
                    Vec::new(),
                );
            }

            let mut local_mutations = Vec::new();
            let (ok, result) = match execute_tool(state, board_id, &tool, &call_input, &mut local_mutations).await {
                Ok(text) => (true, text),
                Err(error) => (false, error.to_string()),
            };

            (
                json!({
                    "index": index,
                    "tool": tool,
                    "ok": ok,
                    "result": result
                }),
                local_mutations,
            )
        }
    });

    let settled = join_all(tasks).await;
    let mut results = Vec::with_capacity(settled.len());
    for (result, local_mutations) in settled {
        mutations.extend(local_mutations);
        results.push(result);
    }

    Ok(json!({ "count": results.len(), "results": results }).to_string())
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
        .get("backgroundColor")
        .or_else(|| input.get("fill"))
        .and_then(|v| v.as_str())
        .unwrap_or("#FFEB3B");
    let stroke = input
        .get("borderColor")
        .or_else(|| input.get("stroke"))
        .and_then(|v| v.as_str())
        .unwrap_or(fill);
    let stroke_width = input
        .get("borderWidth")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            input
                .get("stroke_width")
                .and_then(serde_json::Value::as_f64)
        })
        .unwrap_or(1.0);

    let props = json!({
        "text": text,
        "backgroundColor": fill,
        "fill": fill,
        "borderColor": stroke,
        "stroke": stroke,
        "borderWidth": stroke_width,
        "stroke_width": stroke_width
    });
    let obj = super::object::create_object(state, board_id, "sticky_note", x, y, None, None, 0.0, props, None).await?;
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
    let kind = input
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("rectangle");
    let x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let fill = input
        .get("backgroundColor")
        .or_else(|| input.get("fill"))
        .and_then(|v| v.as_str())
        .unwrap_or("#4CAF50")
        .to_string();
    let stroke = input
        .get("borderColor")
        .or_else(|| input.get("stroke"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fill.clone());
    let stroke_width = input
        .get("borderWidth")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            input
                .get("stroke_width")
                .and_then(serde_json::Value::as_f64)
        })
        .unwrap_or(1.0);

    let props = json!({
        "backgroundColor": fill.clone(),
        "fill": fill,
        "borderColor": stroke.clone(),
        "stroke": stroke,
        "borderWidth": stroke_width,
        "stroke_width": stroke_width
    });
    let w = input.get("width").and_then(serde_json::Value::as_f64);
    let h = input.get("height").and_then(serde_json::Value::as_f64);
    let mut obj = super::object::create_object(state, board_id, kind, x, y, w, h, 0.0, props, None).await?;

    // Update the in-memory object with dimensions.
    if obj.width.is_some() || obj.height.is_some() {
        let mut data = Data::new();
        if let Some(w) = obj.width {
            data.insert("width".into(), json!(w));
        }
        if let Some(h) = obj.height {
            data.insert("height".into(), json!(h));
        }
        obj = super::object::update_object(state, board_id, obj.id, &data, obj.version).await?;
    }

    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("created {kind} shape {id}"))
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

    let props = json!({"title": title});
    let w = input
        .get("width")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(400.0);
    let h = input
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(300.0);
    let obj = super::object::create_object(state, board_id, "frame", x, y, Some(w), Some(h), 0.0, props, None).await?;
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
    let from_id = input.get("fromId").and_then(|v| v.as_str()).unwrap_or("");
    let to_id = input.get("toId").and_then(|v| v.as_str()).unwrap_or("");
    let style = input
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("arrow");

    let props = json!({"source_id": from_id, "target_id": to_id, "style": style});
    // Place connector at origin — rendering uses source/target positions.
    let obj =
        super::object::create_object(state, board_id, "connector", 0.0, 0.0, None, None, 0.0, props, None).await?;
    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("created connector {id} from {from_id} to {to_id}"))
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

    match update_object_with_retry(state, board_id, id, |snapshot| {
        let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
        props.insert("text".into(), json!(new_text));
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

    let background = input
        .get("backgroundColor")
        .or_else(|| input.get("fill"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let border = input
        .get("borderColor")
        .or_else(|| input.get("stroke"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let border_width = input
        .get("borderWidth")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            input
                .get("stroke_width")
                .and_then(serde_json::Value::as_f64)
        });

    if background.is_none() && border.is_none() && border_width.is_none() {
        return Ok("error: provide one of backgroundColor/fill/borderColor/stroke/borderWidth/stroke_width".into());
    }

    match update_object_with_retry(state, board_id, id, |snapshot| {
        let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
        if let Some(width) = border_width {
            props.insert("borderWidth".into(), json!(width));
            props.insert("stroke_width".into(), json!(width));
        }

        let effective_fill = background.clone();
        if let Some(fill) = effective_fill {
            props.insert("backgroundColor".into(), json!(fill.clone()));
            props.insert("fill".into(), json!(fill));
        }

        let effective_stroke = border.clone();
        if let Some(stroke) = effective_stroke {
            props.insert("borderColor".into(), json!(stroke.clone()));
            props.insert("stroke".into(), json!(stroke));
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

async fn execute_apply_changes_yaml(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(yaml_text) = input.get("yaml").and_then(serde_json::Value::as_str) else {
        return Ok("error: missing yaml".into());
    };

    let doc = match parse_yaml_change_document(yaml_text) {
        Ok(doc) => doc,
        Err(err) => return Ok(format!("error: invalid yaml changes document: {err}")),
    };

    let total_ops = doc.changes.create.len() + doc.changes.update.len() + doc.changes.delete.len();
    if total_ops > MAX_YAML_CHANGE_OPS {
        return Ok(format!(
            "error: too many operations ({total_ops}); max is {MAX_YAML_CHANGE_OPS}"
        ));
    }

    let mut created = 0_usize;
    let mut updated = 0_usize;
    let mut deleted = 0_usize;
    let mut errors = Vec::new();

    for change in doc.changes.create {
        let x = match yaml_number(&change.x, "create.x") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let y = match yaml_number(&change.y, "create.y") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let width = match optional_yaml_number(change.width.as_ref(), "create.width") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let height = match optional_yaml_number(change.height.as_ref(), "create.height") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let rotation = match optional_yaml_number(change.rotation.as_ref(), "create.rotation") {
            Ok(v) => v.unwrap_or(0.0),
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let z = match optional_yaml_integer(change.z.as_ref(), "create.z") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };

        let kind = canonical_kind(&change.kind);
        let props = change.props.unwrap_or_else(|| json!({}));
        match super::object::create_object(state, board_id, &kind, x, y, width, height, rotation, props, None).await {
            Ok(mut obj) => {
                if let Some(z_index) = z {
                    let mut update = Data::new();
                    update.insert("z_index".into(), json!(z_index));
                    match super::object::update_object(state, board_id, obj.id, &update, obj.version).await {
                        Ok(updated_obj) => obj = updated_obj,
                        Err(err) => {
                            errors.push(format!("create {} z update failed: {err}", obj.id));
                            continue;
                        }
                    }
                }
                created += 1;
                mutations.push(AiMutation::Created(obj));
            }
            Err(err) => errors.push(format!("create failed: {err}")),
        }
    }

    for change in doc.changes.update {
        let object_id = match change.id.parse::<Uuid>() {
            Ok(id) => id,
            Err(_) => {
                errors.push(format!("update.id invalid uuid: {}", change.id));
                continue;
            }
        };
        let x = match optional_yaml_number(change.x.as_ref(), "update.x") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let y = match optional_yaml_number(change.y.as_ref(), "update.y") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let width = match optional_yaml_number(change.width.as_ref(), "update.width") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let height = match optional_yaml_number(change.height.as_ref(), "update.height") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let rotation = match optional_yaml_number(change.rotation.as_ref(), "update.rotation") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let z = match optional_yaml_integer(change.z.as_ref(), "update.z") {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let props = change.props.clone();

        match update_object_with_retry(state, board_id, object_id, move |_| {
            let mut data = Data::new();
            if let Some(v) = x {
                data.insert("x".into(), json!(v));
            }
            if let Some(v) = y {
                data.insert("y".into(), json!(v));
            }
            if let Some(v) = width {
                data.insert("width".into(), json!(v));
            }
            if let Some(v) = height {
                data.insert("height".into(), json!(v));
            }
            if let Some(v) = rotation {
                data.insert("rotation".into(), json!(v));
            }
            if let Some(v) = z {
                data.insert("z_index".into(), json!(v));
            }
            if let Some(v) = props.clone() {
                data.insert("props".into(), v);
            }
            data
        })
        .await
        {
            Ok(obj) => {
                updated += 1;
                mutations.push(AiMutation::Updated(obj));
            }
            Err(err) => errors.push(format!("update {} failed: {err}", object_id)),
        }
    }

    for change in doc.changes.delete {
        let object_id = match change.id.parse::<Uuid>() {
            Ok(id) => id,
            Err(_) => {
                errors.push(format!("delete.id invalid uuid: {}", change.id));
                continue;
            }
        };
        match super::object::delete_object(state, board_id, object_id).await {
            Ok(()) => {
                deleted += 1;
                mutations.push(AiMutation::Deleted(object_id));
            }
            Err(err) => errors.push(format!("delete {} failed: {err}", object_id)),
        }
    }

    Ok(json!({
        "created": created,
        "updated": updated,
        "deleted": deleted,
        "errors": errors,
    })
    .to_string())
}

fn parse_yaml_change_document(yaml_text: &str) -> Result<YamlChangeDocument, serde_yaml::Error> {
    serde_yaml::from_str::<YamlChangeDocument>(yaml_text)
}

fn canonical_kind(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().as_str() {
        "rect" => "rectangle".to_owned(),
        "circle" => "ellipse".to_owned(),
        "connector" | "arrow" | "line" => "connector".to_owned(),
        other => other.to_owned(),
    }
}

fn optional_yaml_number(value: Option<&serde_yaml::Value>, field: &str) -> Result<Option<f64>, String> {
    value.map(|v| yaml_number(v, field)).transpose()
}

fn yaml_number(value: &serde_yaml::Value, field: &str) -> Result<f64, String> {
    match value {
        serde_yaml::Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| format!("{field} is not a finite number")),
        serde_yaml::Value::String(s) => s
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("{field} must parse as number")),
        _ => Err(format!("{field} must be number or quoted numeric string")),
    }
}

fn optional_yaml_integer(value: Option<&serde_yaml::Value>, field: &str) -> Result<Option<i64>, String> {
    value.map(|v| yaml_integer(v, field)).transpose()
}

fn yaml_integer(value: &serde_yaml::Value, field: &str) -> Result<i64, String> {
    match value {
        serde_yaml::Value::Number(n) => n.as_i64().ok_or_else(|| format!("{field} must be integer")),
        serde_yaml::Value::String(s) => s
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("{field} must parse as integer")),
        _ => Err(format!("{field} must be integer or quoted integer string")),
    }
}

#[cfg(test)]
#[path = "ai_test.rs"]
mod tests;
