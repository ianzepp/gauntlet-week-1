use super::*;

fn make_response(content: serde_json::Value) -> String {
    serde_json::json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": content,
        "model": "claude-sonnet-4-5-20250929",
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 100, "output_tokens": 50 }
    })
    .to_string()
}

#[test]
fn parse_text_response() {
    let json = make_response(serde_json::json!([
        { "type": "text", "text": "Hello world" }
    ]));
    let resp = parse_response(&json).unwrap();
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Hello world"));
    assert_eq!(resp.model, "claude-sonnet-4-5-20250929");
    assert_eq!(resp.stop_reason, "end_turn");
    assert_eq!(resp.input_tokens, 100);
    assert_eq!(resp.output_tokens, 50);
}

#[test]
fn parse_tool_use_response() {
    let json = make_response(serde_json::json!([
        { "type": "tool_use", "id": "tu_1", "name": "create_objects", "input": { "objects": [] } }
    ]));
    let resp = parse_response(&json).unwrap();
    assert_eq!(resp.content.len(), 1);
    assert!(
        matches!(&resp.content[0], ContentBlock::ToolUse { id, name, .. } if id == "tu_1" && name == "create_objects")
    );
}

#[test]
fn parse_mixed_response() {
    let json = make_response(serde_json::json!([
        { "type": "text", "text": "I'll create some notes" },
        { "type": "tool_use", "id": "tu_2", "name": "move_objects", "input": { "moves": [] } }
    ]));
    let resp = parse_response(&json).unwrap();
    assert_eq!(resp.content.len(), 2);
    assert!(matches!(&resp.content[0], ContentBlock::Text { .. }));
    assert!(matches!(&resp.content[1], ContentBlock::ToolUse { .. }));
}

#[test]
fn parse_unknown_content_filtered() {
    let json = make_response(serde_json::json!([
        { "type": "text", "text": "hi" },
        { "type": "some_future_type", "data": {} }
    ]));
    let resp = parse_response(&json).unwrap();
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::Text { .. }));
}

#[test]
fn parse_invalid_json() {
    let result = parse_response("not json");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, LlmError::ApiParse(_)));
}

#[test]
fn parse_thinking_blocks_are_filtered() {
    let json = make_response(serde_json::json!([
        { "type": "thinking", "thinking": "Let me think..." },
        { "type": "text", "text": "Here is my answer" }
    ]));
    let resp = parse_response(&json).unwrap();
    // Thinking blocks should be filtered out, leaving only the text block.
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Here is my answer"));
}

#[test]
fn parse_thinking_only_response_is_empty() {
    let json = make_response(serde_json::json!([
        { "type": "thinking", "thinking": "Let me think..." }
    ]));
    let resp = parse_response(&json).unwrap();
    // If only thinking blocks, content should be empty after filtering.
    assert!(resp.content.is_empty());
}
