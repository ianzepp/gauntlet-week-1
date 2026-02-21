use super::*;
use crate::net::types::FrameStatus;

#[test]
fn build_board_list_request_frame_sets_since_rev_payload() {
    let frame = build_board_list_request_frame(Some("rev-1".to_owned()));
    assert_eq!(frame.syscall, "board:list");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id, None);
    assert_eq!(frame.data, serde_json::json!({ "since_rev": "rev-1" }));
}

#[test]
fn build_board_savepoint_list_request_frame_sets_board_id() {
    let frame = build_board_savepoint_list_request_frame("b1".to_owned());
    assert_eq!(frame.syscall, "board:savepoint:list");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("b1"));
    assert_eq!(frame.data, serde_json::json!({}));
}

#[test]
fn build_board_users_list_request_frame_sets_board_id() {
    let frame = build_board_users_list_request_frame("b2".to_owned());
    assert_eq!(frame.syscall, "board:users:list");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("b2"));
    assert_eq!(frame.data, serde_json::json!({}));
}
