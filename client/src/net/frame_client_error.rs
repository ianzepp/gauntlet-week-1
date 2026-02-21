//! Error frame handling extracted from `frame_client`.

#[cfg(test)]
#[path = "frame_client_error_test.rs"]
mod frame_client_error_test;

#[cfg(feature = "hydrate")]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::boards::BoardsState;

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn apply_board_error_state(syscall: &str, message: &str, boards: &mut BoardsState) -> bool {
    if syscall == "board:list" {
        boards.loading = false;
        boards.error = Some(message.to_owned());
        return true;
    }
    if syscall == "board:create" {
        boards.create_pending = false;
        boards.error = Some(message.to_owned());
        return true;
    }
    if syscall == "board:delete" {
        boards.loading = false;
        boards.error = Some(message.to_owned());
        return true;
    }
    false
}

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
    boards.update(|s| {
        let _ = apply_board_error_state(&frame.syscall, &message, s);
    });
    leptos::logging::warn!("frame error: syscall={} data={}", frame.syscall, frame.data);
    true
}
