//! AI frame handlers extracted from `frame_client`.

#[cfg(test)]
#[path = "frame_client_ai_test.rs"]
mod frame_client_ai_test;

#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::FrameStatus;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::ai::{AiMessage, AiState};

#[cfg(any(test, feature = "hydrate"))]
fn is_user_visible_role(role: &str) -> bool {
    matches!(role, "assistant" | "user" | "error")
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_tool_activity_message(frame: &Frame) -> Option<AiMessage> {
    if frame.syscall != "ai:prompt" || frame.status != FrameStatus::Item {
        return None;
    }
    if frame.data.get("role").and_then(serde_json::Value::as_str) != Some("tool") {
        return None;
    }

    let kind = frame
        .data
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let tool_name = frame
        .data
        .get("tool_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("tool");

    let content = match kind {
        "tool_call" => format!("Running `{tool_name}`..."),
        "tool_result" => {
            let is_error = frame
                .data
                .get("is_error")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if is_error {
                format!("`{tool_name}` failed")
            } else {
                format!("`{tool_name}` completed")
            }
        }
        _ => format!("Tool activity: `{tool_name}`"),
    };

    Some(AiMessage {
        id: frame.id.clone(),
        role: "assistant".to_owned(),
        content,
        timestamp: frame.ts as f64,
        mutations: None,
    })
}

#[cfg(feature = "hydrate")]
pub(super) fn handle_ai_frame(frame: &Frame, ai: leptos::prelude::RwSignal<AiState>) -> bool {
    use leptos::prelude::Update;

    match frame.syscall.as_str() {
        "ai:history" if frame.status == FrameStatus::Done => {
            if let Some(messages) = frame.data.get("messages") {
                let list = messages
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(super::parse_ai_message_value)
                            .filter_map(|mut msg| {
                                if is_user_visible_role(&msg.role) {
                                    return Some(msg);
                                }
                                if msg.role == "tool" {
                                    // History currently stores tool results as free-form content.
                                    // Keep visibility without leaking verbose raw payloads.
                                    msg.role = "assistant".to_owned();
                                    msg.content = "Tool step completed".to_owned();
                                    return Some(msg);
                                }
                                None
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                ai.update(|a| {
                    a.messages = list;
                    a.loading = false;
                });
            }
            true
        }
        "ai:prompt" if frame.status == FrameStatus::Done || frame.status == FrameStatus::Error => {
            if let Some(user_msg) = super::parse_ai_prompt_user_message(frame) {
                ai.update(|a| upsert_ai_user_message(a, user_msg));
            }
            if let Some(msg) = super::parse_ai_prompt_message(frame) {
                ai.update(|a| {
                    if is_user_visible_role(&msg.role) {
                        a.messages.push(msg);
                    }
                    a.loading = false;
                });
            } else if frame.status == FrameStatus::Error {
                let content = super::frame_error_message(frame)
                    .unwrap_or("AI request failed")
                    .to_owned();
                ai.update(|a| {
                    a.messages.push(AiMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        role: "error".to_owned(),
                        content,
                        timestamp: 0.0,
                        mutations: None,
                    });
                    a.loading = false;
                });
            } else {
                ai.update(|a| a.loading = false);
            }
            true
        }
        "ai:prompt" if frame.status == FrameStatus::Item => {
            if let Some(tool_msg) = parse_tool_activity_message(frame) {
                ai.update(|a| {
                    a.messages.push(tool_msg);
                    a.loading = true;
                });
                return true;
            }
            if let Some(msg) = super::parse_ai_prompt_message(frame) {
                ai.update(|a| {
                    if is_user_visible_role(&msg.role) {
                        a.messages.push(msg);
                    }
                    a.loading = true;
                });
            }
            true
        }
        _ => false,
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn upsert_ai_user_message(ai: &mut AiState, msg: AiMessage) {
    if let Some(existing) = ai
        .messages
        .iter_mut()
        .find(|existing| existing.id == msg.id && existing.role == "user")
    {
        existing.content = msg.content;
        if existing.timestamp == 0.0 {
            existing.timestamp = msg.timestamp;
        }
        return;
    }
    ai.messages.push(msg);
}
