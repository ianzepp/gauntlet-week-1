use super::*;
use crate::net::types::FrameStatus;

#[test]
fn request_frame_sets_expected_envelope_fields() {
    let frame = request_frame("object:create", Some("board-1".to_owned()), serde_json::json!({ "id": "o-1" }));

    assert_eq!(frame.syscall, "object:create");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("board-1"));
    assert_eq!(frame.parent_id, None);
    assert_eq!(frame.from, None);
    assert_eq!(frame.ts, 0);
    assert_eq!(frame.data, serde_json::json!({ "id": "o-1" }));
    assert!(!frame.id.is_empty());
}

#[test]
fn request_frame_generates_distinct_ids() {
    let one = request_frame("ping", None, serde_json::json!({}));
    let two = request_frame("ping", None, serde_json::json!({}));
    assert_ne!(one.id, two.id);
}
