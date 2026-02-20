use super::*;
use crate::net::types::FrameStatus;

#[test]
fn object_update_props_frame_builds_expected_payload() {
    let props = serde_json::json!({ "fill": "#112233" });
    let frame = object_update_props_frame("b1", "o1", 7, &props);
    assert_eq!(frame.syscall, "object:update");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("b1"));
    assert_eq!(frame.data["id"], serde_json::json!("o1"));
    assert_eq!(frame.data["version"], serde_json::json!(7));
    assert_eq!(frame.data["props"], props);
}

#[test]
fn object_update_rotation_frame_builds_expected_payload() {
    let frame = object_update_rotation_frame("b1", "o1", 7, 33.5);
    assert_eq!(frame.syscall, "object:update");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("b1"));
    assert_eq!(frame.data["rotation"], serde_json::json!(33.5));
}

#[test]
fn object_update_geometry_frame_builds_expected_payload() {
    let props = serde_json::json!({ "scale": 2.0 });
    let frame = object_update_geometry_frame("b1", "o1", 8, 10.0, 20.0, 30.0, 40.0, &props);
    assert_eq!(frame.syscall, "object:update");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("b1"));
    assert_eq!(frame.data["id"], serde_json::json!("o1"));
    assert_eq!(frame.data["version"], serde_json::json!(8));
    assert_eq!(frame.data["x"], serde_json::json!(10.0));
    assert_eq!(frame.data["y"], serde_json::json!(20.0));
    assert_eq!(frame.data["width"], serde_json::json!(30.0));
    assert_eq!(frame.data["height"], serde_json::json!(40.0));
    assert_eq!(frame.data["props"], props);
}
