use super::*;

// =============================================================
// Helpers
// =============================================================

fn make_frame() -> Frame {
    Frame {
        id: "f-1".to_owned(),
        parent_id: None,
        ts: 1234,
        board_id: Some("b-1".to_owned()),
        from: Some("u-1".to_owned()),
        syscall: "object:create".to_owned(),
        status: FrameStatus::Request,
        trace: None,
        data: serde_json::json!({"kind": "rectangle"}),
    }
}

fn make_board_object() -> BoardObject {
    BoardObject {
        id: "obj-1".to_owned(),
        board_id: "b-1".to_owned(),
        kind: "rectangle".to_owned(),
        x: 10.0,
        y: 20.0,
        width: Some(100.0),
        height: Some(50.0),
        rotation: 0.0,
        z_index: 1,
        props: serde_json::json!({"fill": "#ff0000"}),
        created_by: Some("u-1".to_owned()),
        version: 1,
        group_id: None,
    }
}

fn make_user() -> User {
    User {
        id: "u-1".to_owned(),
        name: "Alice".to_owned(),
        avatar_url: Some("https://example.com/avatar.png".to_owned()),
        color: "#3498db".to_owned(),
        auth_method: "github".to_owned(),
    }
}

// =============================================================
// FrameStatus serde
// =============================================================

#[test]
fn frame_status_serializes_to_lowercase() {
    assert_eq!(serde_json::to_string(&FrameStatus::Request).unwrap(), "\"request\"");
    assert_eq!(serde_json::to_string(&FrameStatus::Item).unwrap(), "\"item\"");
    assert_eq!(serde_json::to_string(&FrameStatus::Bulk).unwrap(), "\"bulk\"");
    assert_eq!(serde_json::to_string(&FrameStatus::Done).unwrap(), "\"done\"");
    assert_eq!(serde_json::to_string(&FrameStatus::Error).unwrap(), "\"error\"");
    assert_eq!(serde_json::to_string(&FrameStatus::Cancel).unwrap(), "\"cancel\"");
}

#[test]
fn frame_status_deserializes_from_lowercase() {
    assert_eq!(
        serde_json::from_str::<FrameStatus>("\"request\"").unwrap(),
        FrameStatus::Request
    );
    assert_eq!(serde_json::from_str::<FrameStatus>("\"item\"").unwrap(), FrameStatus::Item);
    assert_eq!(serde_json::from_str::<FrameStatus>("\"bulk\"").unwrap(), FrameStatus::Bulk);
    assert_eq!(serde_json::from_str::<FrameStatus>("\"done\"").unwrap(), FrameStatus::Done);
    assert_eq!(serde_json::from_str::<FrameStatus>("\"error\"").unwrap(), FrameStatus::Error);
    assert_eq!(serde_json::from_str::<FrameStatus>("\"cancel\"").unwrap(), FrameStatus::Cancel);
}

#[test]
fn frame_status_rejects_uppercase() {
    assert!(serde_json::from_str::<FrameStatus>("\"Request\"").is_err());
}

// =============================================================
// Frame serde round-trip
// =============================================================

#[test]
fn frame_round_trip() {
    let frame = make_frame();
    let json = serde_json::to_string(&frame).unwrap();
    let back: Frame = serde_json::from_str(&json).unwrap();
    assert_eq!(frame, back);
}

#[test]
fn frame_with_all_optional_fields_none() {
    let frame = Frame {
        id: "f-2".to_owned(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "session:connected".to_owned(),
        status: FrameStatus::Done,
        trace: None,
        data: serde_json::Value::Null,
    };
    let json = serde_json::to_string(&frame).unwrap();
    let back: Frame = serde_json::from_str(&json).unwrap();
    assert_eq!(frame, back);
}

#[test]
fn frame_deserializes_from_json_object() {
    let json = r#"{
        "id": "f-3",
        "parent_id": "f-1",
        "ts": 999,
        "board_id": "b-1",
        "from": "u-1",
        "syscall": "chat:send",
        "status": "done",
        "data": {"text": "hello"}
    }"#;
    let frame: Frame = serde_json::from_str(json).unwrap();
    assert_eq!(frame.id, "f-3");
    assert_eq!(frame.parent_id.as_deref(), Some("f-1"));
    assert_eq!(frame.syscall, "chat:send");
    assert_eq!(frame.status, FrameStatus::Done);
    assert_eq!(frame.data["text"], "hello");
}

// =============================================================
// BoardObject serde round-trip
// =============================================================

#[test]
fn board_object_round_trip() {
    let obj = make_board_object();
    let json = serde_json::to_string(&obj).unwrap();
    let back: BoardObject = serde_json::from_str(&json).unwrap();
    assert_eq!(obj, back);
}

#[test]
fn board_object_with_no_optional_fields() {
    let obj = BoardObject {
        id: "obj-2".to_owned(),
        board_id: "b-1".to_owned(),
        kind: "line".to_owned(),
        x: 0.0,
        y: 0.0,
        width: None,
        height: None,
        rotation: 45.0,
        z_index: 0,
        props: serde_json::json!({}),
        created_by: None,
        version: 0,
        group_id: None,
    };
    let json = serde_json::to_string(&obj).unwrap();
    let back: BoardObject = serde_json::from_str(&json).unwrap();
    assert_eq!(obj, back);
}

#[test]
fn board_object_deserializes_integral_float_int_fields() {
    let value = serde_json::json!({
        "id": "obj-3",
        "board_id": "b-1",
        "kind": "rectangle",
        "x": 1.0,
        "y": 2.0,
        "width": 10.0,
        "height": 20.0,
        "rotation": 0.0,
        "z_index": 5.0,
        "props": {},
        "created_by": null,
        "version": 7.0
    });
    let obj: BoardObject = serde_json::from_value(value).unwrap();
    assert_eq!(obj.z_index, 5);
    assert_eq!(obj.version, 7);
}

// =============================================================
// Presence + Point serde round-trip
// =============================================================

#[test]
fn presence_round_trip_with_cursor() {
    let p = Presence {
        client_id: "c-1".to_owned(),
        user_id: "u-1".to_owned(),
        name: "Alice".to_owned(),
        color: "#ff0000".to_owned(),
        cursor: Some(Point { x: 100.0, y: 200.0 }),
        camera_center: Some(Point { x: 300.0, y: 400.0 }),
        camera_zoom: Some(1.25),
        camera_rotation: Some(42.0),
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Presence = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn presence_round_trip_without_cursor() {
    let p = Presence {
        client_id: "c-2".to_owned(),
        user_id: "u-2".to_owned(),
        name: "Bob".to_owned(),
        color: "#00ff00".to_owned(),
        cursor: None,
        camera_center: None,
        camera_zoom: None,
        camera_rotation: None,
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Presence = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn presence_requires_client_id() {
    let json = r##"{
        "user_id": "u-1",
        "name": "Alice",
        "color": "#fff",
        "cursor": null,
        "camera_center": null,
        "camera_zoom": null,
        "camera_rotation": null
    }"##;
    assert!(serde_json::from_str::<Presence>(json).is_err());
}

#[test]
fn point_round_trip() {
    let pt = Point { x: -3.5, y: 42.0 };
    let json = serde_json::to_string(&pt).unwrap();
    let back: Point = serde_json::from_str(&json).unwrap();
    assert_eq!(pt, back);
}

// =============================================================
// User serde round-trip
// =============================================================

#[test]
fn user_round_trip() {
    let u = make_user();
    let json = serde_json::to_string(&u).unwrap();
    let back: User = serde_json::from_str(&json).unwrap();
    assert_eq!(u, back);
}

#[test]
fn user_without_avatar() {
    let u = User {
        id: "u-2".to_owned(),
        name: "Bob".to_owned(),
        avatar_url: None,
        color: "#e74c3c".to_owned(),
        auth_method: "email".to_owned(),
    };
    let json = serde_json::to_string(&u).unwrap();
    let back: User = serde_json::from_str(&json).unwrap();
    assert_eq!(u, back);
}

#[test]
fn user_defaults_auth_method_when_missing() {
    let json = r##"{
        "id": "u-3",
        "name": "Casey",
        "avatar_url": null,
        "color": "#222"
    }"##;
    let user: User = serde_json::from_str(json).unwrap();
    assert_eq!(user.auth_method, "session");
}

// =============================================================
// UserProfile + ProfileStats + SyscallCount serde round-trip
// =============================================================

#[test]
fn user_profile_round_trip() {
    let profile = UserProfile {
        id: "u-1".to_owned(),
        name: "Alice".to_owned(),
        avatar_url: Some("https://example.com/a.png".to_owned()),
        color: "#3498db".to_owned(),
        member_since: Some("2025-01-01".to_owned()),
        stats: ProfileStats {
            total_frames: 100,
            objects_created: 42,
            boards_active: 3,
            last_active: Some("2025-06-15".to_owned()),
            top_syscalls: vec![
                SyscallCount { syscall: "object:create".to_owned(), count: 30 },
                SyscallCount { syscall: "chat:send".to_owned(), count: 20 },
            ],
        },
    };
    let json = serde_json::to_string(&profile).unwrap();
    let back: UserProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(profile, back);
}

#[test]
fn profile_stats_empty_syscalls() {
    let stats =
        ProfileStats { total_frames: 0, objects_created: 0, boards_active: 0, last_active: None, top_syscalls: vec![] };
    let json = serde_json::to_string(&stats).unwrap();
    let back: ProfileStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, back);
}

#[test]
fn syscall_count_round_trip() {
    let sc = SyscallCount { syscall: "object:update".to_owned(), count: 999 };
    let json = serde_json::to_string(&sc).unwrap();
    let back: SyscallCount = serde_json::from_str(&json).unwrap();
    assert_eq!(sc, back);
}
