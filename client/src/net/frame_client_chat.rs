//! Chat frame handlers extracted from `frame_client`.

#[cfg(feature = "hydrate")]
use crate::net::types::Frame;
#[cfg(feature = "hydrate")]
use crate::state::chat::ChatState;

#[cfg(feature = "hydrate")]
pub(super) fn handle_chat_frame(frame: &Frame, chat: leptos::prelude::RwSignal<ChatState>) -> bool {
    use crate::net::types::FrameStatus;
    use leptos::prelude::Update;

    match frame.syscall.as_str() {
        "chat:message" if frame.status == FrameStatus::Done => {
            if let Some(msg) = super::parse_chat_message(frame, &frame.data) {
                chat.update(|c| c.messages.push(msg));
            }
            true
        }
        "chat:history" if frame.status == FrameStatus::Done => {
            if let Some(messages) = frame.data.get("messages") {
                let list = messages
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| super::parse_chat_message(frame, item))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                chat.update(|c| c.messages = list);
            }
            true
        }
        _ => false,
    }
}
