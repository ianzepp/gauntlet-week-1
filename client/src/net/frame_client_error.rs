//! Error frame handling extracted from `frame_client`.

#[cfg(feature = "hydrate")]
use crate::net::types::Frame;
#[cfg(feature = "hydrate")]
use crate::state::boards::BoardsState;

#[cfg(feature = "hydrate")]
pub(super) fn handle_error_frame(frame: &Frame, boards: leptos::prelude::RwSignal<BoardsState>) -> bool {
    use crate::net::types::FrameStatus;
    use leptos::prelude::Update;

    if frame.status != FrameStatus::Error {
        return false;
    }

    let message = super::frame_error_message(frame)
        .unwrap_or("request failed")
        .to_owned();
    if frame.syscall == "board:list" {
        boards.update(|s| {
            s.loading = false;
            s.error = Some(message.clone());
        });
    } else if frame.syscall == "board:create" {
        boards.update(|s| {
            s.create_pending = false;
            s.error = Some(message.clone());
        });
    } else if frame.syscall == "board:delete" {
        boards.update(|s| {
            s.loading = false;
            s.error = Some(message.clone());
        });
    }
    leptos::logging::warn!("frame error: syscall={} data={}", frame.syscall, frame.data);
    true
}
