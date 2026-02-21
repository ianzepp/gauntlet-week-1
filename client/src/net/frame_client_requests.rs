//! Outbound board request helpers extracted from `frame_client`.

#[cfg(test)]
#[path = "frame_client_requests_test.rs"]
mod frame_client_requests_test;

#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::Frame;
#[cfg(feature = "hydrate")]
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use crate::state::boards::BoardsState;

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn build_board_list_request_frame(since_rev: Option<String>) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({
            "since_rev": since_rev
        }),
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn build_board_savepoint_list_request_frame(board_id: String) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "board:savepoint:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn build_board_users_list_request_frame(board_id: String) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "board:users:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    }
}

#[cfg(feature = "hydrate")]
pub(super) fn send_board_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    boards: leptos::prelude::RwSignal<BoardsState>,
) {
    use leptos::prelude::GetUntracked;

    let frame = build_board_list_request_frame(boards.get_untracked().list_rev.clone());
    let _ = super::send_frame(tx, &frame);
}

#[cfg(feature = "hydrate")]
pub(super) fn send_board_savepoint_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    board: leptos::prelude::RwSignal<BoardState>,
) {
    use leptos::prelude::GetUntracked;

    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let frame = build_board_savepoint_list_request_frame(board_id);
    let _ = super::send_frame(tx, &frame);
}

#[cfg(feature = "hydrate")]
pub(super) fn send_board_users_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    board: leptos::prelude::RwSignal<BoardState>,
) {
    use leptos::prelude::GetUntracked;

    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let frame = build_board_users_list_request_frame(board_id);
    let _ = super::send_frame(tx, &frame);
}
