use super::*;

fn sample_frame() -> Frame {
    Frame {
        id: "id-1".to_owned(),
        parent_id: Some("parent-1".to_owned()),
        ts: 42,
        board_id: Some("board-1".to_owned()),
        from: Some("user-1".to_owned()),
        syscall: "object:update".to_owned(),
        status: Status::Done,
        data: serde_json::json!({
            "x": 1.25,
            "ok": true,
            "tags": ["a", "b"],
            "nested": {"k": "v"},
            "nil": null
        }),
    }
}

#[test]
fn status_numeric_mapping_matches_wire_enum() {
    assert_eq!(Status::Request.as_i32(), 0);
    assert_eq!(Status::Done.as_i32(), 1);
    assert_eq!(Status::Error.as_i32(), 2);
    assert_eq!(Status::Cancel.as_i32(), 3);
    assert_eq!(Status::Item.as_i32(), 4);
}

#[test]
fn status_round_trips_from_wire_values() {
    assert_eq!(Status::from_i32(0).expect("status"), Status::Request);
    assert_eq!(Status::from_i32(1).expect("status"), Status::Done);
    assert_eq!(Status::from_i32(2).expect("status"), Status::Error);
    assert_eq!(Status::from_i32(3).expect("status"), Status::Cancel);
    assert_eq!(Status::from_i32(4).expect("status"), Status::Item);
}

#[test]
fn status_from_wire_rejects_out_of_range_value() {
    let err = Status::from_i32(99).expect_err("status should be invalid");
    assert!(matches!(err, CodecError::InvalidStatus(99)));
}

#[test]
fn encode_decode_round_trip_preserves_frame() {
    let frame = sample_frame();
    let bytes = encode_frame(&frame);
    let decoded = decode_frame(&bytes).expect("decode should succeed");
    assert_eq!(decoded, frame);
}

#[test]
fn encode_frame_outputs_non_empty_binary() {
    let frame = sample_frame();
    let bytes = encode_frame(&frame);
    assert!(!bytes.is_empty());
}

#[test]
fn decode_frame_rejects_malformed_bytes() {
    let err = decode_frame(&[0xff, 0x00, 0x01]).expect_err("bytes should fail");
    assert!(matches!(err, CodecError::Decode(_)));
}

#[test]
fn decode_frame_rejects_invalid_wire_status() {
    let wire = WireFrame {
        id: "id-1".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: 77,
        data: Some(json_to_proto_value(&serde_json::json!({}))),
    };
    let mut bytes = Vec::new();
    wire.encode(&mut bytes).expect("encode");

    let err = decode_frame(&bytes).expect_err("status should fail");
    assert!(matches!(err, CodecError::InvalidStatus(77)));
}

#[test]
fn decode_frame_defaults_missing_data_to_empty_object() {
    let wire = WireFrame {
        id: "id-1".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: Status::Request.as_i32(),
        data: None,
    };
    let mut bytes = Vec::new();
    wire.encode(&mut bytes).expect("encode");

    let frame = decode_frame(&bytes).expect("decode");
    assert_eq!(frame.data, serde_json::json!({}));
}

#[test]
fn decode_frame_converts_nan_number_to_json_null() {
    let wire = WireFrame {
        id: "id-1".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: Status::Request.as_i32(),
        data: Some(prost_types::Value {
            kind: Some(prost_types::value::Kind::NumberValue(f64::NAN)),
        }),
    };
    let mut bytes = Vec::new();
    wire.encode(&mut bytes).expect("encode");

    let frame = decode_frame(&bytes).expect("decode");
    assert_eq!(frame.data, Value::Null);
}

#[test]
fn wire_conversion_preserves_empty_optional_fields() {
    let frame = Frame {
        id: String::new(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: String::new(),
        status: Status::Request,
        data: serde_json::json!({}),
    };

    let bytes = encode_frame(&frame);
    let decoded = decode_frame(&bytes).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn nested_payload_round_trips() {
    let frame = Frame {
        id: "id-nested".to_owned(),
        parent_id: Some("p".to_owned()),
        ts: -99,
        board_id: Some("b".to_owned()),
        from: Some("u".to_owned()),
        syscall: "chat:history".to_owned(),
        status: Status::Done,
        data: serde_json::json!({
            "rows": [
                {"id": 1.0, "name": "a"},
                {"id": 2.0, "name": "b"}
            ],
            "meta": {"next": null, "count": 2.0}
        }),
    };

    let bytes = encode_frame(&frame);
    let decoded = decode_frame(&bytes).expect("decode");
    assert_eq!(decoded, frame);
}

#[test]
fn integer_json_numbers_are_normalized_to_float_numbers() {
    let frame = Frame {
        id: "id-int".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: Status::Request,
        data: serde_json::json!({"count": 2}),
    };

    let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
    assert_eq!(decoded.data.get("count"), Some(&serde_json::json!(2.0)));
}

#[test]
fn status_serializes_as_lowercase_json() {
    assert_eq!(
        serde_json::to_string(&Status::Request).expect("serialize"),
        "\"request\""
    );
    assert_eq!(
        serde_json::to_string(&Status::Item).expect("serialize"),
        "\"item\""
    );
    assert_eq!(
        serde_json::to_string(&Status::Cancel).expect("serialize"),
        "\"cancel\""
    );
}

#[test]
fn status_deserializes_from_lowercase_json() {
    assert_eq!(
        serde_json::from_str::<Status>("\"error\"").expect("deserialize"),
        Status::Error
    );
}

#[test]
fn status_rejects_non_lowercase_json() {
    assert!(serde_json::from_str::<Status>("\"Error\"").is_err());
}
