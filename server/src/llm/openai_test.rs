
use super::*;

// ===== chat completions =====

#[test]
fn cc_parse_text_response() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "Hello!" },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
    })
    .to_string();
    let resp = parse_chat_completions_response(&json).unwrap();
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Hello!"));
    assert_eq!(resp.stop_reason, "end_turn");
    assert_eq!(resp.input_tokens, 10);
}

#[test]
fn cc_parse_tool_call() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": { "name": "create_objects", "arguments": "{\"objects\":[]}" }
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": { "prompt_tokens": 20, "completion_tokens": 10 }
    })
    .to_string();
    let resp = parse_chat_completions_response(&json).unwrap();
    assert_eq!(resp.stop_reason, "tool_use");
    assert!(matches!(&resp.content[0], ContentBlock::ToolUse { name, .. } if name == "create_objects"));
}

#[test]
fn cc_parse_missing_choices() {
    let json = serde_json::json!({ "model": "gpt-4o", "choices": [] }).to_string();
    assert!(parse_chat_completions_response(&json).is_err());
}

// ===== responses API =====

#[test]
fn resp_parse_text_response() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "output": [{
            "type": "message",
            "content": [{ "type": "output_text", "text": "Done!" }]
        }],
        "usage": { "input_tokens": 15, "output_tokens": 8 }
    })
    .to_string();
    let resp = parse_responses_response(&json).unwrap();
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Done!"));
    assert_eq!(resp.stop_reason, "end_turn");
}

#[test]
fn resp_parse_function_call() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "output": [{
            "type": "function_call",
            "call_id": "fc_1",
            "name": "move_objects",
            "arguments": "{\"moves\":[]}"
        }],
        "usage": { "input_tokens": 10, "output_tokens": 5 }
    })
    .to_string();
    let resp = parse_responses_response(&json).unwrap();
    assert_eq!(resp.stop_reason, "tool_use");
    assert!(
        matches!(&resp.content[0], ContentBlock::ToolUse { id, name, .. } if id == "fc_1" && name == "move_objects")
    );
}

#[test]
fn resp_parse_output_text_fallback() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "output_text": "Fallback text",
        "usage": { "input_tokens": 5, "output_tokens": 3 }
    })
    .to_string();
    let resp = parse_responses_response(&json).unwrap();
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Fallback text"));
}
