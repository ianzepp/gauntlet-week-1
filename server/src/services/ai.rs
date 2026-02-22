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
//! createSvgObject, updateSvgContent, importSvg, exportSelectionToSvg, deleteObject,
//! moveObject, resizeObject, updateText, changeColor, createAnimationClip, getBoardState.

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
use crate::state::{AppState, BoardObject, ClientViewport};

const DEFAULT_AI_MAX_TOOL_ITERATIONS: usize = 10;
const DEFAULT_AI_MAX_TOKENS: u32 = 4096;
const MAX_SESSION_CONVERSATION_MESSAGES: usize = 20;
const MAX_SVG_BYTES: usize = 200_000;
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

fn elapsed_ms(started_at: Instant) -> i64 {
    i64::try_from(started_at.elapsed().as_millis()).unwrap_or(i64::MAX)
}

fn trace_meta_with_timing(
    trace_id: Uuid,
    span_id: Uuid,
    parent_span_id: Option<Uuid>,
    kind: &str,
    label: Option<&str>,
    root_started_at: Instant,
    duration_ms: Option<i64>,
) -> serde_json::Value {
    let mut trace = trace_meta(trace_id, span_id, parent_span_id, kind, label)
        .as_object()
        .cloned()
        .unwrap_or_default();
    trace.insert("elapsed_ms".into(), json!(elapsed_ms(root_started_at)));
    if let Some(duration_ms) = duration_ms {
        trace.insert("duration_ms".into(), json!(duration_ms));
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
    pub items: Vec<Data>,
    pub trace: AiTraceSummary,
}

#[derive(Debug)]
pub enum AiMutation {
    Created(BoardObject),
    Updated(BoardObject),
    Deleted(Uuid),
}

#[derive(Debug, Clone, Copy)]
pub struct AiTraceSummary {
    pub total_duration_ms: i64,
    pub total_llm_duration_ms: i64,
    pub total_tool_duration_ms: i64,
    pub overhead_duration_ms: i64,
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
    let root_started_at = Instant::now();
    let max_tool_iterations = ai_max_tool_iterations();
    let max_tokens = ai_max_tokens();

    // Rate-limit check: per-client + global request limits.
    state.rate_limiter.check_and_record(client_id)?;

    // Snapshot board objects for context.
    let (board_snapshot, viewport_snapshot) = {
        let boards = state.boards.read().await;
        let board = boards
            .get(&board_id)
            .ok_or(AiError::BoardNotLoaded(board_id))?;
        (
            board.objects.values().cloned().collect::<Vec<_>>(),
            board.viewports.get(&client_id).cloned(),
        )
    };

    let system = build_system_prompt(&board_snapshot, grid_context, viewport_snapshot.as_ref());
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
    let mut stream_items = Vec::new();
    let mut final_text: Option<String> = None;
    let mut total_llm_duration_ms: i64 = 0;
    let mut total_tool_duration_ms: i64 = 0;
    let token_reservation = u64::from(max_tokens);
    let trace_id = trace_id_for_prompt(parent_frame_id);

    for iteration in 0..max_tool_iterations {
        let mut llm_req = Frame::request("ai:llm_request", Data::new())
            .with_board_id(board_id)
            .with_from(user_id.to_string());
        llm_req.parent_id = parent_frame_id;
        let mut llm_req_trace = trace_meta_with_timing(
            trace_id,
            llm_req.id,
            parent_frame_id,
            "ai.llm_request",
            Some("llm"),
            root_started_at,
            None,
        )
        .as_object()
        .cloned()
        .unwrap_or_default();
        llm_req_trace.insert("iteration".into(), json!(iteration));
        llm_req.trace = Some(serde_json::Value::Object(llm_req_trace));
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
                let duration_ms = elapsed_ms(llm_started_at);
                let mut err_frame = llm_req.error_from(&err);
                let mut err_trace = trace_meta_with_timing(
                    trace_id,
                    llm_req.id,
                    parent_frame_id,
                    "ai.llm_request",
                    Some("llm"),
                    root_started_at,
                    Some(duration_ms),
                )
                .as_object()
                .cloned()
                .unwrap_or_default();
                err_trace.insert("iteration".into(), json!(iteration));
                err_frame.trace = Some(serde_json::Value::Object(err_trace));
                super::persistence::enqueue_frame(state, &err_frame);
                state
                    .rate_limiter
                    .release_reserved_tokens(client_id, token_reservation);
                return Err(err.into());
            }
        };
        let duration_ms = elapsed_ms(llm_started_at);
        total_llm_duration_ms = total_llm_duration_ms.saturating_add(duration_ms);
        let total_tokens = response.input_tokens + response.output_tokens;

        let llm_done_data = Data::new();
        let mut llm_trace = trace_meta_with_timing(
            trace_id,
            llm_req.id,
            parent_frame_id,
            "ai.llm_request",
            Some(&response.model),
            root_started_at,
            Some(duration_ms),
        )
        .as_object()
        .cloned()
        .unwrap_or_default();
        llm_trace.insert("iteration".into(), json!(iteration));
        llm_trace.insert("input_tokens".into(), json!(response.input_tokens));
        llm_trace.insert("output_tokens".into(), json!(response.output_tokens));
        llm_trace.insert("tokens".into(), json!(total_tokens));
        llm_trace.insert("stop_reason".into(), json!(response.stop_reason.clone()));
        let mut llm_done = llm_req.done_with(llm_done_data);
        llm_done.trace = Some(serde_json::Value::Object(llm_trace));
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
            let text = text_parts.join("\n");
            final_text = Some(text.clone());
            let mut item = Data::new();
            item.insert("role".into(), json!("assistant"));
            item.insert("content".into(), json!(text));
            item.insert("kind".into(), json!("assistant_text"));
            stream_items.push(item);
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
            let mut start_item = Data::new();
            start_item.insert("role".into(), json!("tool"));
            start_item.insert("kind".into(), json!("tool_call"));
            start_item.insert("tool_use_id".into(), json!(tool_id));
            start_item.insert("tool_name".into(), json!(tool_name));
            start_item.insert("input".into(), input.clone());
            stream_items.push(start_item);

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
                root_started_at,
                &mut total_tool_duration_ms,
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
            let mut result_item = Data::new();
            result_item.insert("role".into(), json!("tool"));
            result_item.insert("kind".into(), json!("tool_result"));
            result_item.insert("tool_use_id".into(), json!(tool_id));
            result_item.insert("tool_name".into(), json!(tool_name));
            result_item.insert("content".into(), json!(content.clone()));
            if let Some(err) = is_error {
                result_item.insert("is_error".into(), json!(err));
            }
            stream_items.push(result_item);
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
        let synthesized = if all_mutations.is_empty() {
            "Done.".into()
        } else {
            format!("Done — {} object(s) updated.", all_mutations.len())
        };
        final_text = Some(synthesized.clone());
        let mut item = Data::new();
        item.insert("role".into(), json!("assistant"));
        item.insert("content".into(), json!(synthesized));
        item.insert("kind".into(), json!("assistant_text"));
        stream_items.push(item);
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

    let total_duration_ms = elapsed_ms(root_started_at);
    let overhead_duration_ms = total_duration_ms
        .saturating_sub(total_llm_duration_ms)
        .saturating_sub(total_tool_duration_ms);
    Ok(AiResult {
        mutations: all_mutations,
        text: final_text,
        items: stream_items,
        trace: AiTraceSummary {
            total_duration_ms,
            total_llm_duration_ms,
            total_tool_duration_ms,
            overhead_duration_ms,
        },
    })
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
    root_started_at: Instant,
    total_tool_duration_ms: &mut i64,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let tool_started_at = Instant::now();
    let syscall = format!("tool:{tool_name}");
    let mut req_data = Data::new();
    req_data.insert("tool_use_id".into(), json!(tool_use_id));
    req_data.insert("input".into(), input.clone());

    let mut req = Frame::request(syscall, req_data)
        .with_board_id(board_id)
        .with_from(user_id.to_string());
    req.parent_id = parent_frame_id;
    let mut req_trace = trace_meta_with_timing(
        trace_id,
        req.id,
        parent_frame_id,
        "ai.tool_call",
        Some(tool_name),
        root_started_at,
        None,
    )
    .as_object()
    .cloned()
    .unwrap_or_default();
    req_trace.insert("iteration".into(), json!(iteration));
    req.trace = Some(serde_json::Value::Object(req_trace));
    super::persistence::enqueue_frame(state, &req);

    match super::tool_syscall::dispatch_tool_frame(state, board_id, &req).await {
        Ok(outcome) => {
            let duration_ms = elapsed_ms(tool_started_at);
            *total_tool_duration_ms = total_tool_duration_ms.saturating_add(duration_ms);
            mutations.extend(outcome.mutations);
            let done_data = outcome.done_data;
            let mut done_trace = trace_meta_with_timing(
                trace_id,
                req.id,
                parent_frame_id,
                "ai.tool_call",
                Some(tool_name),
                root_started_at,
                Some(duration_ms),
            )
            .as_object()
            .cloned()
            .unwrap_or_default();
            done_trace.insert("iteration".into(), json!(iteration));
            let mut done = req.done_with(done_data);
            done.trace = Some(serde_json::Value::Object(done_trace));
            super::persistence::enqueue_frame(state, &done);
            Ok(outcome.content)
        }
        Err(err) => {
            let duration_ms = elapsed_ms(tool_started_at);
            *total_tool_duration_ms = total_tool_duration_ms.saturating_add(duration_ms);
            let mut error = req.error_from(&err);
            error.data.insert("tool_use_id".into(), json!(tool_use_id));
            let mut err_trace = trace_meta_with_timing(
                trace_id,
                req.id,
                parent_frame_id,
                "ai.tool_call",
                Some(tool_name),
                root_started_at,
                Some(duration_ms),
            )
            .as_object()
            .cloned()
            .unwrap_or_default();
            err_trace.insert("iteration".into(), json!(iteration));
            error.trace = Some(serde_json::Value::Object(err_trace));
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

pub(crate) fn build_system_prompt(
    objects: &[BoardObject],
    grid_context: Option<&str>,
    viewport: Option<&ClientViewport>,
) -> String {
    let mut prompt = String::from(BASE_SYSTEM_PROMPT.trim_end());
    prompt.push_str("\n\nBoard context summary:\n");
    let _ = writeln!(prompt, "- total_objects={}", objects.len());
    if objects.is_empty() {
        prompt.push_str("- board_state=empty\n");
    } else {
        let mut by_kind = std::collections::BTreeMap::<&str, usize>::new();
        for obj in objects {
            *by_kind.entry(obj.kind.as_str()).or_default() += 1;
        }
        let mut parts = Vec::with_capacity(by_kind.len());
        for (kind, count) in by_kind {
            parts.push(format!("{kind}:{count}"));
        }
        let _ = writeln!(prompt, "- kind_counts={}", parts.join(", "));
    }
    prompt.push_str("- object_details=not_included_by_default_use_getBoardState_for_details\n");
    append_viewport_context(&mut prompt, viewport);

    if let Some(grid) = grid_context {
        prompt.push('\n');
        prompt.push_str(grid);
        prompt.push('\n');
    }
    prompt
}

fn append_viewport_context(prompt: &mut String, viewport: Option<&ClientViewport>) {
    let Some(view) = viewport else {
        prompt.push_str("- viewer_viewport=unknown\n");
        return;
    };
    let Some(center_x) = view.camera_center_x else {
        prompt.push_str("- viewer_viewport=partial\n");
        return;
    };
    let Some(center_y) = view.camera_center_y else {
        prompt.push_str("- viewer_viewport=partial\n");
        return;
    };
    let Some(zoom) = view.camera_zoom else {
        prompt.push_str("- viewer_viewport=partial\n");
        return;
    };
    let Some(rotation_deg) = view.camera_rotation else {
        prompt.push_str("- viewer_viewport=partial\n");
        return;
    };
    let _ = writeln!(
        prompt,
        "- viewer_center=({center_x:.2}, {center_y:.2}) viewer_zoom={zoom:.4} viewer_rotation_deg={rotation_deg:.2}"
    );
    let Some(viewport_w) = view.camera_viewport_width else {
        prompt.push_str("- viewer_viewport_world=unknown_missing_viewport_dimensions\n");
        return;
    };
    let Some(viewport_h) = view.camera_viewport_height else {
        prompt.push_str("- viewer_viewport_world=unknown_missing_viewport_dimensions\n");
        return;
    };
    let zoom = zoom.max(0.001);
    let world_w = viewport_w / zoom;
    let world_h = viewport_h / zoom;
    let half_w = world_w * 0.5;
    let half_h = world_h * 0.5;
    let radians = rotation_deg.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();

    let corners = [
        (-half_w, -half_h),
        (half_w, -half_h),
        (half_w, half_h),
        (-half_w, half_h),
    ];
    let mut transformed = [(0.0_f64, 0.0_f64); 4];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (index, (dx, dy)) in corners.into_iter().enumerate() {
        let x = center_x + (dx * cos) - (dy * sin);
        let y = center_y + (dx * sin) + (dy * cos);
        transformed[index] = (x, y);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    let _ = writeln!(prompt, "- viewer_viewport_css=({viewport_w:.1}, {viewport_h:.1})");
    let _ = writeln!(prompt, "- viewer_viewport_world=({world_w:.2}, {world_h:.2})");
    let _ = writeln!(
        prompt,
        "- viewer_world_corners=[({:.2}, {:.2}), ({:.2}, {:.2}), ({:.2}, {:.2}), ({:.2}, {:.2})]",
        transformed[0].0,
        transformed[0].1,
        transformed[1].0,
        transformed[1].1,
        transformed[2].0,
        transformed[2].1,
        transformed[3].0,
        transformed[3].1
    );
    let _ = writeln!(prompt, "- viewer_world_aabb=({min_x:.2}, {min_y:.2})..({max_x:.2}, {max_y:.2})");
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
        "createSvgObject" => execute_create_svg_object(state, board_id, input, mutations).await,
        "updateSvgContent" => execute_update_svg_content(state, board_id, input, mutations).await,
        "importSvg" => execute_import_svg(state, board_id, input, mutations).await,
        "exportSelectionToSvg" => execute_export_selection_to_svg(state, board_id, input).await,
        "deleteObject" => execute_delete_object(state, board_id, input, mutations).await,
        "rotateObject" => execute_rotate_object(state, board_id, input, mutations).await,
        "moveObject" => execute_move_object(state, board_id, input, mutations).await,
        "resizeObject" => execute_resize_object(state, board_id, input, mutations).await,
        "updateText" => execute_update_text(state, board_id, input, mutations).await,
        "updateTextStyle" => execute_update_text_style(state, board_id, input, mutations).await,
        "changeColor" => execute_change_color(state, board_id, input, mutations).await,
        "createMermaidDiagram" => execute_create_mermaid_diagram(state, board_id, input, mutations).await,
        "createAnimationClip" => execute_create_animation_clip(state, board_id, input, mutations).await,
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

async fn execute_create_svg_object(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let raw_svg = input
        .get("svg")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if raw_svg.is_empty() {
        return Ok("error: missing svg".into());
    }
    let svg = match sanitize_svg_markup(raw_svg) {
        Ok(s) => s,
        Err(msg) => return Ok(format!("error: {msg}")),
    };
    let Some(x) = input.get("x").and_then(serde_json::Value::as_f64) else {
        return Ok("error: missing x".into());
    };
    let Some(y) = input.get("y").and_then(serde_json::Value::as_f64) else {
        return Ok("error: missing y".into());
    };
    let Some(width) = input.get("width").and_then(serde_json::Value::as_f64) else {
        return Ok("error: missing width".into());
    };
    let Some(height) = input.get("height").and_then(serde_json::Value::as_f64) else {
        return Ok("error: missing height".into());
    };
    let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let view_box = input.get("viewBox").and_then(|v| v.as_str());
    let preserve_aspect_ratio = input.get("preserveAspectRatio").and_then(|v| v.as_str());

    let mut props = serde_json::Map::new();
    props.insert("svg".into(), json!(svg));
    if !title.is_empty() {
        props.insert("title".into(), json!(title));
    }
    if let Some(value) = view_box {
        props.insert("viewBox".into(), json!(value));
    }
    if let Some(value) = preserve_aspect_ratio {
        props.insert("preserveAspectRatio".into(), json!(value));
    }

    let obj = super::object::create_object(
        state,
        board_id,
        "svg",
        x,
        y,
        Some(width.max(1.0)),
        Some(height.max(1.0)),
        0.0,
        serde_json::Value::Object(props),
        None,
        None,
    )
    .await?;
    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("created svg object {id}"))
}

async fn execute_update_svg_content(
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
    let raw_svg = input
        .get("svg")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if raw_svg.is_empty() {
        return Ok("error: missing svg".into());
    }
    let svg = match sanitize_svg_markup(raw_svg) {
        Ok(s) => s,
        Err(msg) => return Ok(format!("error: {msg}")),
    };
    let snapshot = match get_object_snapshot(state, board_id, id).await {
        Ok(obj) => obj,
        Err(e) => {
            warn!(error = %e, %id, "ai: updateSvgContent missing object");
            return Ok(format!("error updating svg on {id}: {e}"));
        }
    };
    if snapshot.kind != "svg" {
        return Ok(format!("error updating svg on {id}: object is not svg"));
    }

    match update_object_with_retry(state, board_id, id, |snapshot| {
        let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
        props.insert("svg".into(), json!(svg));
        let mut data = Data::new();
        data.insert("props".into(), json!(props));
        data
    })
    .await
    {
        Ok(obj) => {
            mutations.push(AiMutation::Updated(obj));
            Ok(format!("updated svg content on {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: updateSvgContent failed");
            Ok(format!("error updating svg on {id}: {e}"))
        }
    }
}

async fn execute_import_svg(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let mode = input
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("single_object");
    if mode != "single_object" {
        return Ok("error: unsupported import mode".into());
    }

    let raw_svg = input
        .get("svg")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if raw_svg.is_empty() {
        return Ok("error: missing svg".into());
    }
    let svg = match sanitize_svg_markup(raw_svg) {
        Ok(s) => s,
        Err(msg) => return Ok(format!("error: {msg}")),
    };

    let x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let scale = input
        .get("scale")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(1.0)
        .clamp(0.1, 10.0);

    let (base_width, base_height) = infer_svg_dimensions(&svg);
    let width = (base_width * scale).max(1.0);
    let height = (base_height * scale).max(1.0);
    let props = json!({ "svg": svg });

    let obj =
        super::object::create_object(state, board_id, "svg", x, y, Some(width), Some(height), 0.0, props, None, None)
            .await?;
    let id = obj.id;
    mutations.push(AiMutation::Created(obj));
    Ok(format!("imported svg as object {id}"))
}

async fn execute_export_selection_to_svg(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
) -> Result<String, AiError> {
    let Some(ids) = input.get("objectIds").and_then(serde_json::Value::as_array) else {
        return Ok("error: missing objectIds".into());
    };
    if ids.is_empty() {
        return Ok("error: objectIds is empty".into());
    }

    let wanted: Vec<Uuid> = ids
        .iter()
        .filter_map(serde_json::Value::as_str)
        .filter_map(|raw| raw.parse::<Uuid>().ok())
        .collect();
    if wanted.is_empty() {
        return Ok("error: objectIds contains no valid UUIDs".into());
    }

    let boards = state.boards.read().await;
    let Some(board) = boards.get(&board_id) else {
        return Ok("error: board not loaded".into());
    };
    let selected: Vec<BoardObject> = wanted
        .iter()
        .filter_map(|id| board.objects.get(id).cloned())
        .collect();
    if selected.is_empty() {
        return Ok("error: no matching objects found".into());
    }

    let min_x = selected.iter().map(|o| o.x).fold(f64::INFINITY, f64::min);
    let min_y = selected.iter().map(|o| o.y).fold(f64::INFINITY, f64::min);
    let max_x = selected
        .iter()
        .map(|o| o.x + o.width.unwrap_or(120.0).max(1.0))
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = selected
        .iter()
        .map(|o| o.y + o.height.unwrap_or(80.0).max(1.0))
        .fold(f64::NEG_INFINITY, f64::max);
    let view_w = (max_x - min_x).max(1.0);
    let view_h = (max_y - min_y).max(1.0);

    let mut body = String::new();
    for obj in &selected {
        let x = obj.x;
        let y = obj.y;
        let width = obj.width.unwrap_or(120.0).max(1.0);
        let height = obj.height.unwrap_or(80.0).max(1.0);
        if obj.kind == "svg"
            && let Some(svg_text) = obj.props.get("svg").and_then(|v| v.as_str())
        {
            let _ = writeln!(
                body,
                "<g transform=\"translate({x:.2},{y:.2})\" data-object-id=\"{}\">{}</g>",
                obj.id, svg_text
            );
            continue;
        }

        let fill = obj
            .props
            .get("fill")
            .and_then(|v| v.as_str())
            .unwrap_or("#D94B4B");
        let stroke = obj
            .props
            .get("stroke")
            .and_then(|v| v.as_str())
            .unwrap_or("#1F1A17");
        let stroke_width = obj
            .props
            .get("strokeWidth")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let _ = writeln!(
            body,
            "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width:.2}\" data-object-id=\"{}\" />",
            obj.id
        );
    }

    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x:.2} {min_y:.2} {view_w:.2} {view_h:.2}\">\n{body}</svg>"
    ))
}

async fn execute_delete_object(
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

    match super::object::delete_object(state, board_id, id).await {
        Ok(()) => {
            mutations.push(AiMutation::Deleted(id));
            Ok(format!("deleted object {id}"))
        }
        Err(e) => {
            warn!(error = %e, %id, "ai: deleteObject failed");
            Ok(format!("error deleting {id}: {e}"))
        }
    }
}

fn infer_svg_dimensions(svg: &str) -> (f64, f64) {
    fn attr_f64(svg: &str, name: &str) -> Option<f64> {
        let needle = format!("{name}=\"");
        let start = svg.find(&needle)? + needle.len();
        let rest = &svg[start..];
        let end = rest.find('"')?;
        let raw = &rest[..end];
        let trimmed = raw.trim_end_matches("px").trim();
        trimmed.parse::<f64>().ok()
    }

    if let (Some(w), Some(h)) = (attr_f64(svg, "width"), attr_f64(svg, "height")) {
        return (w.max(1.0), h.max(1.0));
    }
    if let Some(view_box) = svg.find("viewBox=\"").and_then(|start| {
        let after = &svg[start + "viewBox=\"".len()..];
        let end = after.find('"')?;
        Some(after[..end].to_owned())
    }) {
        let nums: Vec<f64> = view_box
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if nums.len() == 4 {
            return (nums[2].max(1.0), nums[3].max(1.0));
        }
    }
    (320.0, 180.0)
}

fn sanitize_svg_markup(svg: &str) -> Result<String, &'static str> {
    let trimmed = svg.trim();
    if trimmed.is_empty() {
        return Err("missing svg");
    }
    if trimmed.len() > MAX_SVG_BYTES {
        return Err("svg too large");
    }

    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("<svg") {
        return Err("svg must contain <svg root>");
    }
    if lower.contains("<script") {
        return Err("svg contains disallowed script content");
    }
    if lower.contains("javascript:") {
        return Err("svg contains disallowed javascript url");
    }
    if lower.contains("onload=")
        || lower.contains("onerror=")
        || lower.contains("onclick=")
        || lower.contains("onmouseover=")
    {
        return Err("svg contains disallowed event handlers");
    }

    Ok(trimmed.to_owned())
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

async fn execute_create_mermaid_diagram(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let mermaid_text = input.get("mermaid").and_then(|v| v.as_str()).unwrap_or("");
    if mermaid_text.trim().is_empty() {
        return Ok("error: missing mermaid input".into());
    }

    let origin_x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let origin_y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let scale = input
        .get("scale")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(1.0)
        .clamp(0.5, 3.0);

    let diagram = match crate::mermaid::parse(mermaid_text) {
        Ok(d) => d,
        Err(e) => return Ok(format!("error: failed to parse mermaid diagram: {e}")),
    };

    let descriptors = crate::mermaid::render_to_objects(&diagram, origin_x, origin_y, scale);
    if descriptors.is_empty() {
        return Ok("error: mermaid diagram produced no objects".into());
    }

    let mut created_count = 0_usize;
    for desc in &descriptors {
        let props = desc.props.clone();
        let w = if desc.width > 0.0 { Some(desc.width) } else { None };
        let h = if desc.height > 0.0 { Some(desc.height) } else { None };

        match super::object::create_object(state, board_id, &desc.kind, desc.x, desc.y, w, h, 0.0, props, None, None)
            .await
        {
            Ok(obj) => {
                // Ensure dimensions are persisted for shapes that need them.
                let obj = if (desc.kind == "frame" || desc.kind == "rectangle" || desc.kind == "text")
                    && (desc.width > 0.0 || desc.height > 0.0)
                {
                    let mut data = Data::new();
                    if desc.width > 0.0 {
                        data.insert("width".into(), json!(desc.width));
                    }
                    if desc.height > 0.0 {
                        data.insert("height".into(), json!(desc.height));
                    }
                    super::object::update_object(state, board_id, obj.id, &data, obj.version)
                        .await
                        .unwrap_or(obj)
                } else {
                    obj
                };
                mutations.push(AiMutation::Created(obj));
                created_count += 1;
            }
            Err(e) => {
                warn!(error = %e, kind = %desc.kind, "ai: mermaid object creation failed");
            }
        }
    }

    let participant_count = diagram.participants.len();
    let message_count = diagram
        .events
        .iter()
        .filter(|e| matches!(e, crate::mermaid::ast::Event::Message(_)))
        .count();
    Ok(format!(
        "created {created_count} objects from Mermaid diagram ({participant_count} participants, {message_count} messages)"
    ))
}

async fn execute_create_animation_clip(
    state: &AppState,
    board_id: Uuid,
    input: &serde_json::Value,
    mutations: &mut Vec<AiMutation>,
) -> Result<String, AiError> {
    let Some(stream) = input.get("stream").and_then(serde_json::Value::as_array) else {
        return Ok("error: missing stream array".into());
    };
    let target_id_hints: std::collections::HashSet<String> = stream
        .iter()
        .filter_map(|item| item.as_object())
        .filter_map(|ev| {
            let op = ev.get("op").and_then(serde_json::Value::as_str)?;
            let op = op.trim().to_ascii_lowercase();
            if op != "update" && op != "delete" {
                return None;
            }
            ev.get("targetId")
                .or_else(|| ev.get("target_id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    let ordered_target_hints: Vec<String> = stream
        .iter()
        .filter_map(|item| item.as_object())
        .filter_map(|ev| {
            let op = ev.get("op").and_then(serde_json::Value::as_str)?;
            let op = op.trim().to_ascii_lowercase();
            if op != "update" && op != "delete" {
                return None;
            }
            ev.get("targetId")
                .or_else(|| ev.get("target_id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    let single_target_hint = if target_id_hints.len() == 1 {
        target_id_hints.iter().next().cloned()
    } else {
        None
    };

    let mut events = Vec::<serde_json::Value>::new();
    let mut used_create_ids = std::collections::HashSet::<String>::new();
    let mut ordered_target_cursor = 0_usize;
    let mut max_t = 0.0_f64;
    for (index, item) in stream.iter().enumerate() {
        let Some(ev) = item.as_object() else {
            continue;
        };
        let Some(t_ms) = ev
            .get("tMs")
            .or_else(|| ev.get("t_ms"))
            .and_then(serde_json::Value::as_f64)
        else {
            continue;
        };
        let Some(op) = ev.get("op").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let t_ms = t_ms.max(0.0);
        let op = op.trim().to_ascii_lowercase();
        max_t = max_t.max(t_ms);

        match op.as_str() {
            "create" => {
                let Some(object) = ev.get("object") else {
                    continue;
                };
                let Some(object) = normalize_animation_create_object(
                    board_id,
                    object,
                    ev,
                    index,
                    single_target_hint.as_deref(),
                    &ordered_target_hints,
                    &mut ordered_target_cursor,
                    &mut used_create_ids,
                ) else {
                    continue;
                };
                events.push(json!({
                    "tMs": t_ms,
                    "op": "create",
                    "object": object,
                }));
            }
            "update" => {
                let Some(target_id) = ev
                    .get("targetId")
                    .or_else(|| ev.get("target_id"))
                    .and_then(serde_json::Value::as_str)
                else {
                    continue;
                };
                let patch = ev.get("patch").cloned().unwrap_or_else(|| json!({}));
                events.push(json!({
                    "tMs": t_ms,
                    "op": "update",
                    "targetId": target_id,
                    "patch": patch,
                }));
            }
            "delete" => {
                let Some(target_id) = ev
                    .get("targetId")
                    .or_else(|| ev.get("target_id"))
                    .and_then(serde_json::Value::as_str)
                else {
                    continue;
                };
                events.push(json!({
                    "tMs": t_ms,
                    "op": "delete",
                    "targetId": target_id,
                }));
            }
            _ => {}
        }
    }
    if events.is_empty() {
        return Ok("error: stream did not contain valid events".into());
    }
    events.sort_by(|a, b| {
        let ta = a
            .get("tMs")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let tb = b
            .get("tMs")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        ta.total_cmp(&tb)
    });

    let duration_ms = input
        .get("durationMs")
        .or_else(|| input.get("duration_ms"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(max_t + 100.0)
        .max(max_t);
    let looped = input
        .get("loop")
        .or_else(|| input.get("looped"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let scope_object_ids = input
        .get("scopeObjectIds")
        .or_else(|| input.get("scope_object_ids"))
        .and_then(serde_json::Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .filter(|ids| !ids.is_empty());

    let animation = json!({
        "durationMs": duration_ms,
        "loop": looped,
        "scopeObjectIds": scope_object_ids,
        "events": events,
    });

    if let Some(host_object_id) = input
        .get("hostObjectId")
        .and_then(serde_json::Value::as_str)
    {
        let Ok(id) = Uuid::parse_str(host_object_id) else {
            return Ok("error: hostObjectId must be a UUID".into());
        };
        match update_object_with_retry(state, board_id, id, |snapshot| {
            let mut props = snapshot.props.as_object().cloned().unwrap_or_default();
            props.insert("animation".into(), animation.clone());
            let mut data = Data::new();
            data.insert("props".into(), serde_json::Value::Object(props));
            data
        })
        .await
        {
            Ok(obj) => {
                mutations.push(AiMutation::Updated(obj));
                return Ok(format!(
                    "stored animation clip on object {host_object_id} with {} events",
                    events.len()
                ));
            }
            Err(e) => return Ok(format!("error: failed to update host object {host_object_id}: {e}")),
        }
    }

    let title = input
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Animation Clip");
    let x = input
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = input
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let width = input
        .get("width")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(480.0)
        .max(40.0);
    let height = input
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(280.0)
        .max(40.0);
    let props = json!({
        "title": title,
        "stroke": "#1F1A17",
        "strokeWidth": 0.0,
        "animation": animation,
    });
    let host = super::object::create_object(
        state,
        board_id,
        "frame",
        x,
        y,
        Some(width),
        Some(height),
        0.0,
        props,
        None,
        None,
    )
    .await?;
    mutations.push(AiMutation::Created(host.clone()));
    Ok(format!("created animation clip host {} with {} events", host.id, events.len()))
}

fn normalize_animation_create_object(
    board_id: Uuid,
    raw_object: &serde_json::Value,
    event: &serde_json::Map<String, serde_json::Value>,
    index: usize,
    single_target_hint: Option<&str>,
    ordered_target_hints: &[String],
    ordered_target_cursor: &mut usize,
    used_ids: &mut std::collections::HashSet<String>,
) -> Option<serde_json::Value> {
    let source = raw_object.as_object()?;

    let mut id = source
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            event
                .get("targetId")
                .or_else(|| event.get("target_id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| single_target_hint.map(str::to_owned))
        .or_else(|| next_unclaimed_target_hint(ordered_target_hints, ordered_target_cursor, used_ids))
        .unwrap_or_else(|| format!("anim_obj_{}_{}", index, Uuid::new_v4().simple()));
    if used_ids.contains(&id) {
        id = format!("{id}_{index}");
    }
    used_ids.insert(id.clone());

    let kind = source
        .get("kind")
        .or_else(|| source.get("type"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("rectangle")
        .to_owned();
    let x = source
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = source
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let width = source
        .get("width")
        .and_then(serde_json::Value::as_f64)
        .map_or(serde_json::Value::Null, |v| json!(v));
    let height = source
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .map_or(serde_json::Value::Null, |v| json!(v));
    let rotation = source
        .get("rotation")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let z_index = source
        .get("z_index")
        .or_else(|| source.get("zIndex"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let version = source
        .get("version")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let group_id = source
        .get("group_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let created_by = source
        .get("created_by")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let object_board_id = source
        .get("board_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| board_id.to_string());

    let props = if let Some(props) = source.get("props").and_then(serde_json::Value::as_object) {
        serde_json::Value::Object(props.clone())
    } else {
        let mut props = serde_json::Map::new();
        for key in [
            "fill",
            "stroke",
            "strokeWidth",
            "text",
            "title",
            "fontSize",
            "textColor",
            "svg",
            "viewBox",
            "preserveAspectRatio",
            "a",
            "b",
            "fromId",
            "toId",
            "style",
        ] {
            if let Some(v) = source.get(key) {
                props.insert(key.to_owned(), v.clone());
            }
        }
        if (kind == "line" || kind == "arrow") && !props.contains_key("a") && !props.contains_key("b") {
            let w = source
                .get("width")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(120.0);
            let h = source
                .get("height")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            props.insert("a".into(), json!({ "x": x, "y": y }));
            props.insert("b".into(), json!({ "x": x + w, "y": y + h }));
        }
        serde_json::Value::Object(props)
    };

    Some(json!({
        "id": id,
        "board_id": object_board_id,
        "kind": kind,
        "x": x,
        "y": y,
        "width": width,
        "height": height,
        "rotation": rotation,
        "z_index": z_index,
        "props": props,
        "created_by": created_by,
        "version": version,
        "group_id": group_id,
    }))
}

fn next_unclaimed_target_hint(
    ordered_target_hints: &[String],
    cursor: &mut usize,
    used_ids: &std::collections::HashSet<String>,
) -> Option<String> {
    while *cursor < ordered_target_hints.len() {
        let candidate = ordered_target_hints[*cursor].clone();
        *cursor += 1;
        if !used_ids.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
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
