use super::*;
use crate::net::types::FrameStatus;

#[test]
fn build_board_list_frame_includes_since_rev() {
    let frame = build_board_list_frame(Some("rev-123".to_owned()));
    assert_eq!(frame.syscall, "board:list");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id, None);
    assert_eq!(frame.data, serde_json::json!({ "since_rev": "rev-123" }));
}

#[test]
fn build_board_create_frame_sets_name_payload() {
    let frame = build_board_create_frame("Project board");
    assert_eq!(frame.syscall, "board:create");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.data, serde_json::json!({ "name": "Project board" }));
}

#[test]
fn build_board_delete_frame_sets_board_id_payload() {
    let frame = build_board_delete_frame("b-1");
    assert_eq!(frame.syscall, "board:delete");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.data, serde_json::json!({ "board_id": "b-1" }));
}

#[test]
fn build_access_redeem_frame_sets_code_payload() {
    let frame = build_access_redeem_frame("ABC123");
    assert_eq!(frame.syscall, "board:access:redeem");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.data, serde_json::json!({ "code": "ABC123" }));
}
