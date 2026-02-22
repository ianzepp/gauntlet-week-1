use super::*;

#[test]
fn request_sets_fields() {
    let frame = Frame::request("board:create", Data::new());
    assert_eq!(frame.syscall, "board:create");
    assert_eq!(frame.status, Status::Request);
    assert!(frame.parent_id.is_none());
    assert!(frame.board_id.is_none());
    assert!(frame.ts > 0);
}

#[test]
fn reply_inherits_context() {
    let board_id = Uuid::new_v4();
    let req = Frame::request("object:create", Data::new()).with_board_id(board_id);
    let done = req.done();

    assert_eq!(done.parent_id, Some(req.id));
    assert_eq!(done.board_id, Some(board_id));
    assert_eq!(done.syscall, "object:create");
    assert_eq!(done.status, Status::Done);
}

#[test]
fn done_is_terminal() {
    assert!(Status::Done.is_terminal());
    assert!(Status::Error.is_terminal());
    assert!(Status::Cancel.is_terminal());
    assert!(!Status::Request.is_terminal());
    assert!(!Status::Item.is_terminal());
    assert!(!Status::Bulk.is_terminal());
}

#[test]
fn prefix_extraction() {
    let frame = Frame::request("object:create", Data::new());
    assert_eq!(frame.prefix(), "object");

    let frame = Frame::request("noseparator", Data::new());
    assert_eq!(frame.prefix(), "noseparator");
}

#[test]
fn json_round_trip() {
    let board_id = Uuid::new_v4();
    let original = Frame::request("board:join", Data::new())
        .with_board_id(board_id)
        .with_from("test-user")
        .with_data("key", "value");

    let json = serde_json::to_string(&original).expect("serialize");
    let restored: Frame = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.id, original.id);
    assert_eq!(restored.board_id, Some(board_id));
    assert_eq!(restored.syscall, "board:join");
    assert_eq!(restored.from.as_deref(), Some("test-user"));
    assert_eq!(restored.data.get("key").and_then(|v| v.as_str()), Some("value"));
}

#[test]
fn error_from_typed() {
    #[derive(Debug, thiserror::Error)]
    #[error("not found")]
    struct NotFound;

    impl ErrorCode for NotFound {
        fn error_code(&self) -> &'static str {
            "E_NOT_FOUND"
        }
    }

    let req = Frame::request("object:get", Data::new());
    let err = req.error_from(&NotFound);

    assert_eq!(err.status, Status::Error);
    assert_eq!(err.data.get("code").and_then(|v| v.as_str()), Some("E_NOT_FOUND"));
    assert_eq!(err.data.get("message").and_then(|v| v.as_str()), Some("not found"));
    assert_eq!(
        err.data
            .get("retryable")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
}

#[test]
fn cancel_references_target() {
    let req = Frame::request("ai:prompt", Data::new());
    let cancel = Frame::cancel(req.id);

    assert_eq!(cancel.parent_id, Some(req.id));
    assert_eq!(cancel.status, Status::Cancel);
    assert!(cancel.status.is_terminal());
}

#[test]
fn deserialize_client_cursor_frame() {
    // Exact JSON shape the client sends for cursor:moved.
    let json = r#"{
            "id": "053ffe5e-16ed-41f1-a36d-eabdd40c0ceb",
            "parent_id": null,
            "ts": 1739750400000,
            "board_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            "from": null,
            "syscall": "cursor:moved",
            "status": "request",
            "data": { "x": 100.5, "y": 200.3, "name": "Alice" }
        }"#;
    let frame: Frame = serde_json::from_str(json).expect("cursor frame should deserialize");
    assert_eq!(frame.syscall, "cursor:moved");
    assert_eq!(frame.status, Status::Request);
    assert!(frame.board_id.is_some());
    assert!(frame.from.is_none());
}

#[test]
fn deserialize_client_frame_null_board_id() {
    // Client sends board_id: null before joining a board.
    let json = r#"{
            "id": "053ffe5e-16ed-41f1-a36d-eabdd40c0ceb",
            "parent_id": null,
            "ts": 1739750400000,
            "board_id": null,
            "from": null,
            "syscall": "board:list",
            "status": "request",
            "data": {}
        }"#;
    let frame: Frame = serde_json::from_str(json).expect("null board_id should deserialize");
    assert!(frame.board_id.is_none());
}

#[test]
fn deserialize_client_frame_string_board_id() {
    // Client sends board_id as a UUID string.
    let json = r#"{
            "id": "053ffe5e-16ed-41f1-a36d-eabdd40c0ceb",
            "parent_id": null,
            "ts": 1739750400000,
            "board_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            "from": null,
            "syscall": "object:create",
            "status": "request",
            "data": {}
        }"#;
    let frame: Frame = serde_json::from_str(json).expect("string board_id should deserialize");
    assert!(frame.board_id.is_some());
}

#[test]
fn deserialize_minimal_frame() {
    // Only id and syscall — all other fields should default.
    let json = r#"{"id": "053ffe5e-16ed-41f1-a36d-eabdd40c0ceb", "syscall": "board:list"}"#;
    let frame: Frame = serde_json::from_str(json).expect("minimal frame should deserialize");
    assert_eq!(frame.syscall, "board:list");
    assert_eq!(frame.status, Status::Request);
    assert!(frame.board_id.is_none());
    assert!(frame.from.is_none());
    assert!(frame.data.is_empty());
}

#[test]
fn deserialize_client_frame_empty_string_board_id_fails() {
    // Client might send board_id: "" — this must fail Uuid parse.
    let json = r#"{
            "id": "053ffe5e-16ed-41f1-a36d-eabdd40c0ceb",
            "parent_id": null,
            "ts": 1739750400000,
            "board_id": "",
            "from": null,
            "syscall": "cursor:moved",
            "status": "request",
            "data": {}
        }"#;
    let result = serde_json::from_str::<Frame>(json);
    assert!(result.is_err(), "empty string board_id should fail deserialization");
}

#[test]
fn deserialize_client_frame_empty_string_from_ok() {
    // Client might send from: "" — should this work?
    let json = r#"{
            "id": "053ffe5e-16ed-41f1-a36d-eabdd40c0ceb",
            "parent_id": null,
            "ts": 1739750400000,
            "board_id": null,
            "from": "",
            "syscall": "board:list",
            "status": "request",
            "data": {}
        }"#;
    let frame: Frame = serde_json::from_str(json).expect("empty string from should deserialize");
    assert_eq!(frame.from, Some(String::new()));
}

// =============================================================================
// Builder chains
// =============================================================================

#[test]
fn with_content_sets_content_key() {
    let frame = Frame::request("chat:message", Data::new()).with_content("Hello world");
    assert_eq!(frame.data.get(FRAME_CONTENT).and_then(|v| v.as_str()), Some("Hello world"));
}

#[test]
fn with_data_multiple_keys() {
    let frame = Frame::request("test:multi", Data::new())
        .with_data("a", "1")
        .with_data("b", "2");
    assert_eq!(frame.data.get("a").and_then(|v| v.as_str()), Some("1"));
    assert_eq!(frame.data.get("b").and_then(|v| v.as_str()), Some("2"));
}

#[test]
fn with_data_overwrites_same_key() {
    let frame = Frame::request("test:overwrite", Data::new())
        .with_data("key", "first")
        .with_data("key", "second");
    assert_eq!(frame.data.get("key").and_then(|v| v.as_str()), Some("second"));
}

#[test]
fn with_from_sets_from_field() {
    let frame = Frame::request("test:from", Data::new()).with_from("user-42");
    assert_eq!(frame.from.as_deref(), Some("user-42"));
}

// =============================================================================
// done_with carries data
// =============================================================================

#[test]
fn done_with_carries_data() {
    let req = Frame::request("board:create", Data::new());
    let mut data = Data::new();
    data.insert("board_id".into(), serde_json::Value::String("abc".into()));
    let done = req.done_with(data);
    assert_eq!(done.status, Status::Done);
    assert_eq!(done.data.get("board_id").and_then(|v| v.as_str()), Some("abc"));
}

// =============================================================================
// error frame
// =============================================================================

#[test]
fn error_sets_message_key() {
    let req = Frame::request("object:get", Data::new());
    let err = req.error("not found");
    assert_eq!(err.status, Status::Error);
    assert_eq!(err.data.get(FRAME_MESSAGE).and_then(|v| v.as_str()), Some("not found"));
}

// =============================================================================
// error_from with retryable error
// =============================================================================

#[test]
fn error_from_retryable_error() {
    #[derive(Debug, thiserror::Error)]
    #[error("transient failure")]
    struct Transient;

    impl ErrorCode for Transient {
        fn error_code(&self) -> &'static str {
            "E_TRANSIENT"
        }
        fn retryable(&self) -> bool {
            true
        }
    }

    let req = Frame::request("ai:prompt", Data::new());
    let err = req.error_from(&Transient);
    assert_eq!(err.data.get(FRAME_RETRYABLE).and_then(|v| v.as_bool()), Some(true));
    assert_eq!(err.data.get(FRAME_CODE).and_then(|v| v.as_str()), Some("E_TRANSIENT"));
}

// =============================================================================
// Status serde all variants
// =============================================================================

#[test]
fn status_serde_all_variants() {
    for (status, expected) in [
        (Status::Request, "\"request\""),
        (Status::Item, "\"item\""),
        (Status::Bulk, "\"bulk\""),
        (Status::Done, "\"done\""),
        (Status::Error, "\"error\""),
        (Status::Cancel, "\"cancel\""),
    ] {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, expected, "serialization of {status:?}");
        let restored: Status = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, status, "deserialization of {expected}");
    }
}

// =============================================================================
// prefix edge cases
// =============================================================================

#[test]
fn prefix_empty_syscall() {
    let frame = Frame::request("", Data::new());
    assert_eq!(frame.prefix(), "");
}

#[test]
fn prefix_multiple_colons() {
    let frame = Frame::request("a:b:c", Data::new());
    assert_eq!(frame.prefix(), "a");
}
