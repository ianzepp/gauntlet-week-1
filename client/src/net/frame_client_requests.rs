//! Outbound board request helpers extracted from `frame_client`.

#[cfg(feature = "hydrate")]
use crate::net::types::Frame;
#[cfg(feature = "hydrate")]
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use crate::state::boards::BoardsState;

#[cfg(feature = "hydrate")]
pub(super) fn send_board_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    boards: leptos::prelude::RwSignal<BoardsState>,
) {
    use leptos::prelude::GetUntracked;

    let since_rev = boards.get_untracked().list_rev;
    let frame = Frame {
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
    };
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
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "board:savepoint:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    };
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
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "board:users:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    };
    let _ = super::send_frame(tx, &frame);
}
