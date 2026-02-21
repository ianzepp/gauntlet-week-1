use super::*;
use crate::net::types::{Frame, FrameStatus};

fn frame(syscall: &str, status: FrameStatus, data: serde_json::Value) -> Frame {
    Frame {
        id: "f1".to_owned(),
        parent_id: None,
        ts: 42,
        board_id: Some("b1".to_owned()),
        from: Some("u1".to_owned()),
        syscall: syscall.to_owned(),
        status,
        data,
    }
}

#[test]
fn parse_done_chat_message_frame_parses_message_payload() {
    let f = frame(
        "chat:message",
        FrameStatus::Done,
        serde_json::json!({
            "id": "m1",
            "content": "hello",
            "user_id": "u1",
            "user_name": "Ann",
            "user_color": "#112233",
            "timestamp": 11
        }),
    );
    let msg = parse_done_chat_message_frame(&f).expect("message frame should parse");
    assert_eq!(msg.id, "m1");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.user_name, "Ann");
    assert_eq!(msg.timestamp, 11.0);
}

#[test]
fn parse_done_chat_message_frame_rejects_non_done_or_other_syscall() {
    let f1 = frame("chat:message", FrameStatus::Request, serde_json::json!({}));
    let f2 = frame("chat:history", FrameStatus::Done, serde_json::json!({}));
    assert!(parse_done_chat_message_frame(&f1).is_none());
    assert!(parse_done_chat_message_frame(&f2).is_none());
}

#[test]
fn parse_done_chat_history_frame_parses_valid_items_and_skips_invalid() {
    let f = frame(
        "chat:history",
        FrameStatus::Done,
        serde_json::json!({
            "messages": [
                {"id":"m1","content":"one","user_id":"u1","user_name":"Ann","user_color":"#111","timestamp":1},
                {"id":"m2","user_id":"u1"}
            ]
        }),
    );
    let items = parse_done_chat_history_frame(&f).expect("history frame should parse");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "m1");
}

#[test]
fn parse_done_chat_history_frame_requires_messages_field() {
    let f = frame("chat:history", FrameStatus::Done, serde_json::json!({}));
    assert!(parse_done_chat_history_frame(&f).is_none());
}
