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

#[test]
fn apply_object_frame_delete_clears_selected_object() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.selection.insert(obj.id.clone());
    board.objects.insert(obj.id.clone(), obj);

    apply_object_frame(
        &frame("object:delete", FrameStatus::Done, serde_json::json!({ "id": "obj-1" })),
        &mut board,
    );

    assert!(!board.objects.contains_key("obj-1"));
    assert!(!board.selection.contains("obj-1"));
}

#[test]
fn apply_object_frame_update_unknown_clears_stale_selection() {
    let mut board = crate::state::board::BoardState::default();
    board.selection.insert("obj-404".to_owned());

    apply_object_frame(
        &frame(
            "object:update",
            FrameStatus::Done,
            serde_json::json!({ "id": "obj-404", "x": 42.0 }),
        ),
        &mut board,
    );

    assert!(!board.selection.contains("obj-404"));
}

#[test]
fn apply_cursor_moved_supports_server_shape() {
    let mut board = crate::state::board::BoardState::default();
    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "client_id": "c-1",
            "x": 12.5,
            "y": -7.25,
            "name": "Alice",
            "color": "#22c55e"
        }),
    );

    let p = board.presence.get("client:c-1").expect("cursor presence");
    assert_eq!(p.name, "Alice");
    assert_eq!(p.color, "#22c55e");
    let cursor = p.cursor.as_ref().expect("cursor point");
    assert_eq!(cursor.x, 12.5);
    assert_eq!(cursor.y, -7.25);
}

#[test]
fn apply_cursor_moved_updates_existing_presence_by_name_and_color() {
    let mut board = crate::state::board::BoardState::default();
    board.presence.insert(
        "u-1".to_owned(),
        crate::net::types::Presence {
            user_id: "u-1".to_owned(),
            name: "Alice".to_owned(),
            color: "#22c55e".to_owned(),
            cursor: None,
        },
    );

    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "client_id": "c-1",
            "x": 50.0,
            "y": 60.0,
            "name": "Alice",
            "color": "#22c55e"
        }),
    );

    assert!(!board.presence.contains_key("client:c-1"));
    let p = board.presence.get("u-1").expect("existing presence");
    let cursor = p.cursor.as_ref().expect("cursor point");
    assert_eq!(cursor.x, 50.0);
    assert_eq!(cursor.y, 60.0);
}

#[test]
fn apply_object_frame_drag_updates_object_geometry() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.objects.insert(obj.id.clone(), obj);

    apply_object_frame(
        &frame(
            "object:drag",
            FrameStatus::Request,
            serde_json::json!({
                "id": "obj-1",
                "x": 200.0,
                "y": 300.0,
                "width": 150.0,
                "height": 90.0,
                "rotation": 45.0
            }),
        ),
        &mut board,
    );

    let obj = board.objects.get("obj-1").expect("object should exist");
    assert_eq!(obj.x, 200.0);
    assert_eq!(obj.y, 300.0);
    assert_eq!(obj.width, Some(150.0));
    assert_eq!(obj.height, Some(90.0));
    assert_eq!(obj.rotation, 45.0);
}
