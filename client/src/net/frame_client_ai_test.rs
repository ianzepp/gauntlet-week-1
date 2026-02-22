use super::*;

fn msg(id: &str, role: &str, content: &str, timestamp: f64) -> AiMessage {
    AiMessage { id: id.to_owned(), role: role.to_owned(), content: content.to_owned(), timestamp, mutations: None }
}

#[test]
fn upsert_ai_user_message_updates_existing_user_message_content() {
    let mut ai =
        AiState { messages: vec![msg("m1", "user", "old", 0.0), msg("m2", "assistant", "reply", 10.0)], loading: true };

    upsert_ai_user_message(&mut ai, msg("m1", "user", "new", 42.0));

    assert_eq!(ai.messages.len(), 2);
    assert_eq!(ai.messages[0].content, "new");
    assert_eq!(ai.messages[0].timestamp, 42.0);
}

#[test]
fn upsert_ai_user_message_preserves_existing_nonzero_timestamp() {
    let mut ai = AiState { messages: vec![msg("m1", "user", "old", 7.0)], loading: false };

    upsert_ai_user_message(&mut ai, msg("m1", "user", "new", 99.0));

    assert_eq!(ai.messages[0].content, "new");
    assert_eq!(ai.messages[0].timestamp, 7.0);
}

#[test]
fn upsert_ai_user_message_appends_when_id_not_found() {
    let mut ai = AiState::default();
    upsert_ai_user_message(&mut ai, msg("m1", "user", "hello", 1.0));
    assert_eq!(ai.messages.len(), 1);
    assert_eq!(ai.messages[0].id, "m1");
}

#[test]
fn user_visible_role_filters_tool_messages() {
    assert!(is_user_visible_role("assistant"));
    assert!(is_user_visible_role("user"));
    assert!(is_user_visible_role("error"));
    assert!(!is_user_visible_role("tool"));
}

fn frame_with_tool_item(kind: &str, tool_name: &str, is_error: Option<bool>) -> Frame {
    let mut data = serde_json::json!({
        "role": "tool",
        "kind": kind,
        "tool_name": tool_name
    });
    if let Some(flag) = is_error {
        data["is_error"] = serde_json::json!(flag);
    }
    Frame {
        id: "f-tool".to_owned(),
        parent_id: None,
        ts: 123,
        board_id: None,
        from: None,
        syscall: "ai:prompt".to_owned(),
        status: crate::net::types::FrameStatus::Item,
        trace: None,
        data,
    }
}

#[test]
fn parse_tool_activity_message_summarizes_tool_call() {
    let frame = frame_with_tool_item("tool_call", "createSvgObject", None);
    let msg = parse_tool_activity_message(&frame).expect("tool activity message");
    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.content, "Running `createSvgObject`...");
}

#[test]
fn parse_tool_activity_message_summarizes_tool_result() {
    let frame = frame_with_tool_item("tool_result", "createSvgObject", Some(false));
    let msg = parse_tool_activity_message(&frame).expect("tool activity message");
    assert_eq!(msg.content, "`createSvgObject` completed");

    let frame_err = frame_with_tool_item("tool_result", "createSvgObject", Some(true));
    let msg_err = parse_tool_activity_message(&frame_err).expect("tool activity message");
    assert_eq!(msg_err.content, "`createSvgObject` failed");
}
