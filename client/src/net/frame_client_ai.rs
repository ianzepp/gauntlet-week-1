//! AI frame handlers extracted from `frame_client`.

#[cfg(test)]
#[path = "frame_client_ai_test.rs"]
mod frame_client_ai_test;

#[cfg(feature = "hydrate")]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::ai::{AiMessage, AiState};

#[cfg(feature = "hydrate")]
pub(super) fn handle_ai_frame(frame: &Frame, ai: leptos::prelude::RwSignal<AiState>) -> bool {
    use crate::net::types::FrameStatus;
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
                    a.messages.push(msg);
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
