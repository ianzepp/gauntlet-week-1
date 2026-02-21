//! Chat frame handlers extracted from `frame_client`.

#[cfg(test)]
#[path = "frame_client_chat_test.rs"]
mod frame_client_chat_test;

#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::FrameStatus;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::chat::ChatMessage;
#[cfg(feature = "hydrate")]
use crate::state::chat::ChatState;

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_done_chat_message_frame(frame: &Frame) -> Option<ChatMessage> {
    if frame.syscall != "chat:message" || frame.status != FrameStatus::Done {
        return None;
    }
    super::parse_chat_message(frame, &frame.data)
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_done_chat_history_frame(frame: &Frame) -> Option<Vec<ChatMessage>> {
    if frame.syscall != "chat:history" || frame.status != FrameStatus::Done {
        return None;
    }
    let messages = frame.data.get("messages")?;
    let list = messages
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| super::parse_chat_message(frame, item))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(list)
}

#[cfg(feature = "hydrate")]
pub(super) fn handle_chat_frame(frame: &Frame, chat: leptos::prelude::RwSignal<ChatState>) -> bool {
    use leptos::prelude::Update;

    if let Some(msg) = parse_done_chat_message_frame(frame) {
        chat.update(|c| c.messages.push(msg));
        return true;
    }

    if let Some(list) = parse_done_chat_history_frame(frame) {
        chat.update(|c| c.messages = list);
        return true;
    }

    false
}
