use super::*;
use crate::net::types::{Frame, FrameStatus};

fn frame_with(data: serde_json::Value) -> Frame {
    Frame {
        id: "f1".to_owned(),
        parent_id: None,
        ts: 123,
        board_id: Some("b1".to_owned()),
        from: Some("u1".to_owned()),
        syscall: "test".to_owned(),
        status: FrameStatus::Done,
        data,
    }
}

#[test]
fn pick_str_returns_first_matching_string_key() {
    let data = serde_json::json!({ "a": 1, "b": "two", "c": "three" });
    assert_eq!(pick_str(&data, &["x", "b", "c"]), Some("two"));
    assert_eq!(pick_str(&data, &["x", "y"]), None);
}

#[test]
fn pick_number_supports_float_and_integer_values() {
    let data = serde_json::json!({ "float": 1.5, "int": 7 });
    assert_eq!(pick_number(&data, &["float"]), Some(1.5));
    assert_eq!(pick_number(&data, &["int"]), Some(7.0));
    assert_eq!(pick_number(&data, &["missing"]), None);
}

#[test]
fn frame_error_message_prefers_message_then_error() {
    let one = frame_with(serde_json::json!({ "message": "m1", "error": "e1" }));
    let two = frame_with(serde_json::json!({ "error": "e1" }));
    let three = frame_with(serde_json::json!({}));
    assert_eq!(frame_error_message(&one), Some("m1"));
    assert_eq!(frame_error_message(&two), Some("e1"));
    assert_eq!(frame_error_message(&three), None);
}

#[test]
fn parse_ai_message_value_reads_integer_mutations() {
    let parsed = parse_ai_message_value(&serde_json::json!({
        "id": "m1",
        "role": "assistant",
        "content": "ok",
        "mutations": 4
    }))
    .expect("message should parse");
    assert_eq!(parsed.id, "m1");
    assert_eq!(parsed.role, "assistant");
    assert_eq!(parsed.content, "ok");
    assert_eq!(parsed.mutations, Some(4));
}

#[test]
fn parse_ai_message_value_accepts_integral_float_mutations_only() {
    let integral = parse_ai_message_value(&serde_json::json!({
        "content": "ok",
        "mutations": 3.0
    }))
    .expect("message should parse");
    assert_eq!(integral.mutations, Some(3));

    let fractional = parse_ai_message_value(&serde_json::json!({
        "content": "ok",
        "mutations": 3.5
    }))
    .expect("message should parse");
    assert_eq!(fractional.mutations, None);
}
