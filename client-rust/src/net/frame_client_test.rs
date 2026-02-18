use super::*;
use crate::net::types::{BoardObject, Frame, FrameStatus};

fn frame(syscall: &str, status: FrameStatus, data: serde_json::Value) -> Frame {
    Frame {
        id: "f-1".to_owned(),
        parent_id: None,
        ts: 123,
        board_id: Some("b-1".to_owned()),
        from: Some("u-1".to_owned()),
        syscall: syscall.to_owned(),
        status,
        data,
    }
}

fn object() -> BoardObject {
    BoardObject {
        id: "obj-1".to_owned(),
        board_id: "b-1".to_owned(),
        kind: "rectangle".to_owned(),
        x: 0.0,
        y: 0.0,
        width: Some(100.0),
        height: Some(80.0),
        rotation: 0.0,
        z_index: 1,
        props: serde_json::json!({"fill":"#fff"}),
        created_by: Some("u-1".to_owned()),
        version: 1,
    }
}

#[test]
fn frame_error_message_prefers_message_then_error() {
    let f = frame(
        "board:list",
        FrameStatus::Error,
        serde_json::json!({"message":"m1","error":"m2"}),
    );
    assert_eq!(frame_error_message(&f), Some("m1"));

    let f = frame("board:list", FrameStatus::Error, serde_json::json!({"error":"m2"}));
    assert_eq!(frame_error_message(&f), Some("m2"));
}

#[test]
fn parse_chat_message_uses_fallback_fields() {
    let f = frame("chat:message", FrameStatus::Done, serde_json::json!({}));
    let data = serde_json::json!({
        "message": "hello",
        "from": "u-9",
        "ts": 777
    });
    let msg = parse_chat_message(&f, &data).expect("chat message");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.user_id, "u-9");
    assert_eq!(msg.timestamp, 777.0);
}

#[test]
fn parse_ai_message_value_rejects_empty_content() {
    let data = serde_json::json!({"role":"assistant","text":"   "});
    assert!(parse_ai_message_value(&data).is_none());
}

#[test]
fn parse_ai_prompt_message_uses_error_role_and_ts() {
    let f = frame("ai:prompt", FrameStatus::Error, serde_json::json!({"content":"failed"}));
    let msg = parse_ai_prompt_message(&f).expect("ai message");
    assert_eq!(msg.role, "error");
    assert_eq!(msg.content, "failed");
    assert_eq!(msg.timestamp, 123.0);
}

#[test]
fn merge_object_update_applies_known_fields() {
    let mut obj = object();
    merge_object_update(
        &mut obj,
        &serde_json::json!({
            "x": 15.0,
            "y": 25.0,
            "width": 120.0,
            "height": 64.0,
            "rotation": 30.0,
            "z_index": 9,
            "version": 3,
            "props": {"k":"v"}
        }),
    );

    assert_eq!(obj.x, 15.0);
    assert_eq!(obj.y, 25.0);
    assert_eq!(obj.width, Some(120.0));
    assert_eq!(obj.height, Some(64.0));
    assert_eq!(obj.rotation, 30.0);
    assert_eq!(obj.z_index, 9);
    assert_eq!(obj.version, 3);
    assert_eq!(obj.props, serde_json::json!({"k":"v"}));
}
