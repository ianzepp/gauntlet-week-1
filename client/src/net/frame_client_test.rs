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

fn presence(client_id: &str, user_id: &str, name: &str, color: &str) -> crate::net::types::Presence {
    crate::net::types::Presence {
        client_id: client_id.to_owned(),
        user_id: user_id.to_owned(),
        name: name.to_owned(),
        color: color.to_owned(),
        cursor: None,
        camera_center: None,
        camera_zoom: None,
        camera_rotation: None,
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
fn parse_chat_message_rejects_missing_content() {
    let f = frame("chat:message", FrameStatus::Done, serde_json::json!({}));
    let data = serde_json::json!({
        "user_id": "u-9",
        "timestamp": 123
    });
    assert!(parse_chat_message(&f, &data).is_none());
}

#[test]
fn parse_ai_message_value_rejects_empty_content() {
    let data = serde_json::json!({"role":"assistant","text":"   "});
    assert!(parse_ai_message_value(&data).is_none());
}

#[test]
fn parse_ai_message_value_uses_ts_fallback() {
    let data = serde_json::json!({"role":"assistant","content":"ok","ts":999});
    let msg = parse_ai_message_value(&data).expect("ai message");
    assert_eq!(msg.timestamp, 999.0);
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
fn parse_ai_prompt_message_returns_none_when_payload_has_no_text() {
    let f = frame("ai:prompt", FrameStatus::Done, serde_json::json!({}));
    assert!(parse_ai_prompt_message(&f).is_none());
}

#[test]
fn parse_ai_prompt_user_message_reads_prompt() {
    let f = frame("ai:prompt", FrameStatus::Done, serde_json::json!({"prompt":"draft release notes"}));
    let msg = parse_ai_prompt_user_message(&f).expect("user prompt message");
    assert_eq!(msg.id, "f-1");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "draft release notes");
    assert_eq!(msg.timestamp, 123.0);
}

#[test]
fn parse_ai_prompt_user_message_prefers_parent_id_for_reconciliation() {
    let mut f = frame("ai:prompt", FrameStatus::Done, serde_json::json!({"prompt":"draft release notes"}));
    f.id = "reply-id".to_owned();
    f.parent_id = Some("request-id".to_owned());
    let msg = parse_ai_prompt_user_message(&f).expect("user prompt message");
    assert_eq!(msg.id, "request-id");
}

#[test]
fn parse_ai_prompt_user_message_rejects_blank_prompt() {
    let f = frame("ai:prompt", FrameStatus::Done, serde_json::json!({"prompt":"   "}));
    assert!(parse_ai_prompt_user_message(&f).is_none());
}

#[test]
fn upsert_ai_user_message_updates_existing_pending_row() {
    let mut state = crate::state::ai::AiState {
        messages: vec![crate::state::ai::AiMessage {
            id: "f-1".to_owned(),
            role: "user".to_owned(),
            content: "draft".to_owned(),
            timestamp: 0.0,
            mutations: None,
        }],
        loading: true,
    };

    upsert_ai_user_message(
        &mut state,
        crate::state::ai::AiMessage {
            id: "f-1".to_owned(),
            role: "user".to_owned(),
            content: "draft release notes".to_owned(),
            timestamp: 123.0,
            mutations: None,
        },
    );

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].content, "draft release notes");
    assert_eq!(state.messages[0].timestamp, 123.0);
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
fn merge_object_update_accepts_integral_float_int_fields() {
    let mut obj = object();
    merge_object_update(
        &mut obj,
        &serde_json::json!({
            "z_index": 12.0,
            "version": 8.0
        }),
    );
    assert_eq!(obj.z_index, 12);
    assert_eq!(obj.version, 8);
}

#[test]
fn parse_board_objects_accepts_integral_float_int_fields() {
    let data = serde_json::json!({
        "objects": [
            {
                "id": "obj-1",
                "board_id": "b-1",
                "kind": "rectangle",
                "x": 10.0,
                "y": 20.0,
                "width": 100.0,
                "height": 80.0,
                "rotation": 0.0,
                "z_index": 5.0,
                "props": {"fill":"#fff"},
                "created_by": "u-1",
                "version": 3.0
            }
        ]
    });

    let objects = parse_board_objects(&data).expect("board objects");
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].z_index, 5);
    assert_eq!(objects[0].version, 3);
}

#[test]
fn parse_board_object_item_accepts_integral_float_int_fields() {
    let data = serde_json::json!({
        "id": "obj-2",
        "board_id": "b-1",
        "kind": "rectangle",
        "x": 1.0,
        "y": 2.0,
        "width": 10.0,
        "height": 20.0,
        "rotation": 0.0,
        "z_index": 6.0,
        "props": {"fill":"#fff"},
        "created_by": "u-1",
        "version": 4.0
    });
    let obj = parse_board_object_item(&data).expect("board object item");
    assert_eq!(obj.id, "obj-2");
    assert_eq!(obj.z_index, 6);
    assert_eq!(obj.version, 4);
}

#[test]
fn parse_board_list_items_keeps_snapshot_geometry() {
    let data = serde_json::json!({
        "boards": [
            {
                "id": "b-1",
                "name": "Alpha",
                "snapshot": [
                    {
                        "kind": "sticky_note",
                        "x": 10.0,
                        "y": 20.0,
                        "width": 100.0,
                        "height": 80.0,
                        "rotation": 15.0,
                        "z_index": 3
                    }
                ]
            }
        ]
    });

    let items = parse_board_list_items(&data);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "b-1");
    assert_eq!(items[0].name, "Alpha");
    assert_eq!(items[0].snapshot.len(), 1);
    assert_eq!(items[0].snapshot[0].kind, "sticky_note");
    assert_eq!(items[0].snapshot[0].z_index, 3);
}

#[test]
fn parse_board_list_items_tolerates_partial_snapshot_rows() {
    let data = serde_json::json!({
        "boards": [
            {
                "id": "b-1",
                "name": "Alpha",
                "snapshot": [
                    {
                        "kind": "sticky_note",
                        "x": 10.0,
                        "y": 20.0,
                        "z_index": 1
                    },
                    {
                        "kind": "sticky_note"
                    }
                ]
            }
        ]
    });

    let items = parse_board_list_items(&data);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].snapshot.len(), 1);
}

#[test]
fn deleted_board_id_prefers_payload_then_board_id_fallback() {
    let frame_with_payload = frame(
        "board:delete",
        FrameStatus::Done,
        serde_json::json!({"board_id":"b-from-payload"}),
    );
    assert_eq!(deleted_board_id(&frame_with_payload).as_deref(), Some("b-from-payload"));

    let frame_with_board_id = Frame {
        id: "f-2".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: Some("b-from-frame".to_owned()),
        from: None,
        syscall: "board:delete".to_owned(),
        status: FrameStatus::Done,
        data: serde_json::json!({}),
    };
    assert_eq!(deleted_board_id(&frame_with_board_id).as_deref(), Some("b-from-frame"));
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
        100,
    );

    let p = board.presence.get("c-1").expect("cursor presence");
    assert_eq!(p.name, "Alice");
    assert_eq!(p.color, "#22c55e");
    let cursor = p.cursor.as_ref().expect("cursor point");
    assert_eq!(cursor.x, 12.5);
    assert_eq!(cursor.y, -7.25);
}

#[test]
fn apply_cursor_moved_updates_existing_presence_by_name_and_color() {
    let mut board = crate::state::board::BoardState::default();
    board
        .presence
        .insert("u-1".to_owned(), presence("u-1", "u-1", "Alice", "#22c55e"));

    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "client_id": "c-1",
            "x": 50.0,
            "y": 60.0,
            "name": "Alice",
            "color": "#22c55e"
        }),
        120,
    );

    let p = board.presence.get("c-1").expect("existing presence");
    let cursor = p.cursor.as_ref().expect("cursor point");
    assert_eq!(cursor.x, 50.0);
    assert_eq!(cursor.y, 60.0);
}

#[test]
fn apply_cursor_clear_removes_cursor_for_client_presence() {
    let mut board = crate::state::board::BoardState::default();
    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "client_id": "c-9",
            "x": 1.0,
            "y": 2.0,
            "name": "Agent",
            "color": "#fff"
        }),
        100,
    );

    apply_cursor_clear(&mut board, &serde_json::json!({ "client_id": "c-9" }));
    let p = board.presence.get("c-9").expect("presence");
    assert!(p.cursor.is_none());
    assert!(!board.cursor_updated_at.contains_key("c-9"));
}

#[test]
fn cleanup_stale_cursors_clears_old_cursor_points() {
    let mut board = crate::state::board::BoardState::default();
    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "client_id": "c-2",
            "x": 10.0,
            "y": 20.0,
            "name": "Agent",
            "color": "#fff"
        }),
        100,
    );

    cleanup_stale_cursors(&mut board, 5000);
    let p = board.presence.get("c-2").expect("presence");
    assert!(p.cursor.is_none());
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

    let obj = board
        .drag_objects
        .get("obj-1")
        .expect("drag shadow should exist");
    assert_eq!(obj.x, 200.0);
    assert_eq!(obj.y, 300.0);
    assert_eq!(obj.width, Some(150.0));
    assert_eq!(obj.height, Some(90.0));
    assert_eq!(obj.rotation, 45.0);

    let base = board
        .objects
        .get("obj-1")
        .expect("authoritative object should exist");
    assert_eq!(base.x, 0.0);
    assert_eq!(base.y, 0.0);
}

#[test]
fn apply_object_frame_drag_end_clears_drag_shadow() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.objects.insert(obj.id.clone(), obj.clone());
    board.drag_objects.insert(obj.id.clone(), obj);
    board.drag_updated_at.insert("obj-1".to_owned(), 100);

    apply_object_frame(
        &frame("object:drag:end", FrameStatus::Request, serde_json::json!({ "id": "obj-1" })),
        &mut board,
    );

    assert!(!board.drag_objects.contains_key("obj-1"));
    assert!(!board.drag_updated_at.contains_key("obj-1"));
}

#[test]
fn apply_object_frame_drag_is_ignored_for_locally_selected_object() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.selection.insert(obj.id.clone());
    board.objects.insert(obj.id.clone(), obj);

    apply_object_frame(
        &frame(
            "object:drag",
            FrameStatus::Request,
            serde_json::json!({
                "id": "obj-1",
                "x": 200.0,
                "y": 300.0
            }),
        ),
        &mut board,
    );

    assert!(!board.drag_objects.contains_key("obj-1"));
}

#[test]
fn cleanup_stale_drags_removes_expired_entries() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.objects.insert(obj.id.clone(), obj.clone());
    board.drag_objects.insert(obj.id.clone(), obj);
    board.drag_updated_at.insert("obj-1".to_owned(), 100);

    cleanup_stale_drags(&mut board, 1700);

    assert!(!board.drag_objects.contains_key("obj-1"));
    assert!(!board.drag_updated_at.contains_key("obj-1"));
}

#[test]
fn apply_object_frame_drag_smooths_from_previous_drag_state() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.objects.insert(obj.id.clone(), obj.clone());
    board
        .drag_objects
        .insert(obj.id.clone(), crate::net::types::BoardObject { x: 100.0, y: 100.0, ..obj });
    board.drag_updated_at.insert("obj-1".to_owned(), 100);

    apply_object_frame(
        &Frame {
            ts: 250,
            ..frame(
                "object:drag",
                FrameStatus::Request,
                serde_json::json!({
                    "id": "obj-1",
                    "x": 200.0,
                    "y": 200.0
                }),
            )
        },
        &mut board,
    );

    let dragged = board.drag_objects.get("obj-1").expect("drag object");
    assert!(dragged.x > 100.0 && dragged.x < 200.0);
    assert!(dragged.y > 100.0 && dragged.y < 200.0);
}

#[test]
fn apply_object_frame_drag_fast_updates_use_raw_values() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.objects.insert(obj.id.clone(), obj.clone());
    board
        .drag_objects
        .insert(obj.id.clone(), crate::net::types::BoardObject { x: 100.0, y: 100.0, ..obj });
    board.drag_updated_at.insert("obj-1".to_owned(), 200);

    apply_object_frame(
        &Frame {
            ts: 240,
            ..frame(
                "object:drag",
                FrameStatus::Request,
                serde_json::json!({
                    "id": "obj-1",
                    "x": 200.0,
                    "y": 200.0
                }),
            )
        },
        &mut board,
    );

    let dragged = board.drag_objects.get("obj-1").expect("drag object");
    assert_eq!(dragged.x, 200.0);
    assert_eq!(dragged.y, 200.0);
}

#[test]
fn apply_cursor_moved_ignores_events_without_client_id() {
    let mut board = crate::state::board::BoardState::default();
    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "x": 50.0,
            "y": 60.0
        }),
        120,
    );
    assert!(board.presence.is_empty());
}

#[test]
fn apply_cursor_moved_sets_camera_fields_when_present() {
    let mut board = crate::state::board::BoardState::default();
    apply_cursor_moved(
        &mut board,
        &serde_json::json!({
            "client_id": "c-3",
            "x": 1.0,
            "y": 2.0,
            "camera_center_x": 10.0,
            "camera_center_y": 20.0,
            "camera_zoom": 2.0,
            "camera_rotation": 33.0
        }),
        200,
    );

    let p = board.presence.get("c-3").expect("presence");
    let center = p.camera_center.as_ref().expect("camera center");
    assert_eq!(center.x, 10.0);
    assert_eq!(center.y, 20.0);
    assert_eq!(p.camera_zoom, Some(2.0));
    assert_eq!(p.camera_rotation, Some(33.0));
}

#[test]
fn apply_cursor_clear_is_noop_when_client_missing() {
    let mut board = crate::state::board::BoardState::default();
    board
        .presence
        .insert("c-1".to_owned(), presence("c-1", "u-1", "Alice", "#fff"));

    apply_cursor_clear(&mut board, &serde_json::json!({}));

    assert!(board.presence.contains_key("c-1"));
}

#[test]
fn cleanup_helpers_ignore_non_positive_timestamps() {
    let mut board = crate::state::board::BoardState::default();
    let obj = object();
    board.drag_objects.insert(obj.id.clone(), obj);
    board.drag_updated_at.insert("obj-1".to_owned(), 100);
    board
        .presence
        .insert("c-1".to_owned(), presence("c-1", "u-1", "Alice", "#fff"));
    board.cursor_updated_at.insert("c-1".to_owned(), 100);

    cleanup_stale_drags(&mut board, 0);
    cleanup_stale_cursors(&mut board, -1);

    assert!(board.drag_objects.contains_key("obj-1"));
    assert!(board.cursor_updated_at.contains_key("c-1"));
}

#[test]
fn smoothing_thresholds_cover_edges() {
    assert!(!should_smooth_drag(100, 179));
    assert!(should_smooth_drag(100, 180));
    assert_eq!(smoothing_alpha(100, 180), 0.45);
    assert_eq!(smoothing_alpha(100, 220), 0.55);
    assert_eq!(smoothing_alpha(100, 320), 0.65);
}

#[test]
fn pick_helpers_return_none_for_missing_keys() {
    let payload = serde_json::json!({"foo":"bar","n":10});
    assert_eq!(pick_str(&payload, &["missing"]), None);
    assert_eq!(pick_number(&payload, &["missing"]), None);
}
