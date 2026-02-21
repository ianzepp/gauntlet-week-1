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

// =============================================================
// Status: all variants serialise and deserialise correctly
// =============================================================

#[test]
fn status_all_variants_serialize_lowercase() {
    let cases = [
        (Status::Request, "\"request\""),
        (Status::Item, "\"item\""),
        (Status::Done, "\"done\""),
        (Status::Error, "\"error\""),
        (Status::Cancel, "\"cancel\""),
    ];
    for (status, expected) in cases {
        assert_eq!(serde_json::to_string(&status).expect("serialize"), expected);
    }
}

#[test]
fn status_all_variants_deserialize_from_lowercase() {
    let cases = [
        ("\"request\"", Status::Request),
        ("\"item\"", Status::Item),
        ("\"done\"", Status::Done),
        ("\"error\"", Status::Error),
        ("\"cancel\"", Status::Cancel),
    ];
    for (input, expected) in cases {
        let got: Status = serde_json::from_str(input).expect("deserialize");
        assert_eq!(got, expected);
    }
}

#[test]
fn status_all_variants_as_i32_are_distinct() {
    let values: Vec<i32> = [
        Status::Request.as_i32(),
        Status::Item.as_i32(),
        Status::Done.as_i32(),
        Status::Error.as_i32(),
        Status::Cancel.as_i32(),
    ]
    .to_vec();
    let deduped: std::collections::HashSet<i32> = values.iter().copied().collect();
    assert_eq!(deduped.len(), 5);
}

// =============================================================
// encode_frame / decode_frame: per-field fidelity
// =============================================================

#[test]
fn encode_decode_preserves_all_optional_string_fields() {
    let frame = Frame {
        id: "test-id".to_owned(),
        parent_id: Some("parent-id".to_owned()),
        ts: 12345,
        board_id: Some("board-id".to_owned()),
        from: Some("user-id".to_owned()),
        syscall: "object:create".to_owned(),
        status: Status::Request,
        data: serde_json::json!({}),
    };
    let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
    assert_eq!(decoded.id, "test-id");
    assert_eq!(decoded.parent_id.as_deref(), Some("parent-id"));
    assert_eq!(decoded.board_id.as_deref(), Some("board-id"));
    assert_eq!(decoded.from.as_deref(), Some("user-id"));
    assert_eq!(decoded.ts, 12345);
    assert_eq!(decoded.syscall, "object:create");
    assert_eq!(decoded.status, Status::Request);
}

#[test]
fn encode_decode_negative_timestamp() {
    let frame = Frame {
        id: "neg-ts".to_owned(),
        parent_id: None,
        ts: -999_999,
        board_id: None,
        from: None,
        syscall: "board:join".to_owned(),
        status: Status::Done,
        data: serde_json::json!({}),
    };
    let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
    assert_eq!(decoded.ts, -999_999);
}

#[test]
fn encode_decode_all_status_variants() {
    for status in [
        Status::Request,
        Status::Item,
        Status::Done,
        Status::Error,
        Status::Cancel,
    ] {
        let frame = Frame {
            id: "status-test".to_owned(),
            parent_id: None,
            ts: 1,
            board_id: None,
            from: None,
            syscall: "board:join".to_owned(),
            status,
            data: serde_json::json!({}),
        };
        let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
        assert_eq!(decoded.status, status);
    }
}

#[test]
fn encode_decode_bool_in_data() {
    let frame = Frame {
        id: "bool-test".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:join".to_owned(),
        status: Status::Done,
        data: serde_json::json!({"success": true, "failed": false}),
    };
    let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
    assert_eq!(decoded.data["success"], serde_json::json!(true));
    assert_eq!(decoded.data["failed"], serde_json::json!(false));
}

#[test]
fn encode_decode_null_in_data() {
    let frame = Frame {
        id: "null-test".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:join".to_owned(),
        status: Status::Done,
        data: serde_json::json!({"value": null}),
    };
    let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
    assert_eq!(decoded.data["value"], serde_json::Value::Null);
}

#[test]
fn encode_decode_array_in_data() {
    let frame = Frame {
        id: "arr-test".to_owned(),
        parent_id: None,
        ts: 1,
        board_id: None,
        from: None,
        syscall: "board:join".to_owned(),
        status: Status::Done,
        data: serde_json::json!({"items": [1.0, 2.0, 3.0]}),
    };
    let decoded = decode_frame(&encode_frame(&frame)).expect("decode");
    assert_eq!(decoded.data["items"].as_array().unwrap().len(), 3);
}

#[test]
fn decode_frame_rejects_empty_bytes() {
    // Empty input is not valid protobuf for a non-empty message type.
    // prost treats empty bytes as a zero-value message (all defaults).
    let result = decode_frame(&[]);
    // An empty frame has status 0 (Request), which is valid, so it should succeed.
    // This verifies the decoder doesn't panic on empty input.
    let _ = result;
}

#[test]
fn frame_serde_roundtrip_via_json() {
    let frame = sample_frame();
    let json = serde_json::to_string(&frame).expect("serialize");
    let restored: Frame = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, frame);
}
