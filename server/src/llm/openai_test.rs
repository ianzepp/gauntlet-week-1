use super::*;
use crate::llm::types::{Content, ContentBlock, Message};

// ===== chat completions parsing =====

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

#[test]
fn cc_parse_length_finish_reason() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "truncated" },
            "finish_reason": "length"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
    })
    .to_string();
    let resp = parse_chat_completions_response(&json).unwrap();
    assert_eq!(resp.stop_reason, "max_tokens");
}

#[test]
fn cc_parse_empty_content_with_tool_calls() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "id": "call_2",
                    "type": "function",
                    "function": { "name": "foo", "arguments": "{}" }
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": { "prompt_tokens": 5, "completion_tokens": 3 }
    })
    .to_string();
    let resp = parse_chat_completions_response(&json).unwrap();
    assert_eq!(resp.stop_reason, "tool_use");
    assert_eq!(resp.content.len(), 1);
    assert!(matches!(&resp.content[0], ContentBlock::ToolUse { name, .. } if name == "foo"));
}

#[test]
fn cc_parse_missing_usage() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "hi" },
            "finish_reason": "stop"
        }]
    })
    .to_string();
    let resp = parse_chat_completions_response(&json).unwrap();
    assert_eq!(resp.input_tokens, 0);
    assert_eq!(resp.output_tokens, 0);
}

// ===== responses API parsing =====

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

#[test]
fn resp_parse_max_tokens_via_incomplete_details() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "output": [{
            "type": "message",
            "content": [{ "type": "output_text", "text": "partial" }]
        }],
        "incomplete_details": { "reason": "max_output_tokens" },
        "usage": { "input_tokens": 10, "output_tokens": 5 }
    })
    .to_string();
    let resp = parse_responses_response(&json).unwrap();
    assert_eq!(resp.stop_reason, "max_tokens");
}

#[test]
fn resp_parse_function_call_with_id_fallback() {
    let json = serde_json::json!({
        "model": "gpt-4o",
        "output": [{
            "type": "function_call",
            "id": "fallback_id",
            "name": "test_tool",
            "arguments": "{}"
        }],
        "usage": { "input_tokens": 5, "output_tokens": 3 }
    })
    .to_string();
    let resp = parse_responses_response(&json).unwrap();
    assert!(matches!(&resp.content[0], ContentBlock::ToolUse { id, .. } if id == "fallback_id"));
}

// ===== build_chat_completions_messages =====

#[test]
fn build_cc_messages_adds_system_message() {
    let msgs = build_chat_completions_messages("You are helpful.", &[]);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].role, "system");
    assert_eq!(msgs[0].content.as_deref(), Some("You are helpful."));
}

#[test]
fn build_cc_messages_skips_empty_system() {
    let msgs = build_chat_completions_messages("  ", &[]);
    assert!(msgs.is_empty());
}

#[test]
fn build_cc_messages_maps_text_messages() {
    let messages = vec![
        Message { role: "user".into(), content: Content::Text("hello".into()) },
        Message { role: "assistant".into(), content: Content::Text("hi".into()) },
    ];
    let msgs = build_chat_completions_messages("sys", &messages);
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[1].role, "user");
    assert_eq!(msgs[1].content.as_deref(), Some("hello"));
    assert_eq!(msgs[2].role, "assistant");
    assert_eq!(msgs[2].content.as_deref(), Some("hi"));
}

#[test]
fn build_cc_messages_maps_tool_use_blocks() {
    let messages = vec![Message {
        role: "assistant".into(),
        content: Content::Blocks(vec![ContentBlock::ToolUse {
            id: "call_1".into(),
            name: "test_fn".into(),
            input: serde_json::json!({"key": "val"}),
        }]),
    }];
    let msgs = build_chat_completions_messages("", &messages);
    assert_eq!(msgs.len(), 1);
    let tool_calls = msgs[0].tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_1");
    assert_eq!(tool_calls[0].function.name, "test_fn");
}

#[test]
fn build_cc_messages_maps_tool_result_blocks() {
    let messages = vec![Message {
        role: "user".into(),
        content: Content::Blocks(vec![ContentBlock::ToolResult {
            tool_use_id: "call_1".into(),
            content: "result data".into(),
            is_error: None,
        }]),
    }];
    let msgs = build_chat_completions_messages("", &messages);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].role, "tool");
    assert_eq!(msgs[0].content.as_deref(), Some("result data"));
    assert_eq!(msgs[0].tool_call_id.as_deref(), Some("call_1"));
}

#[test]
fn build_cc_messages_skips_thinking_and_unknown_blocks() {
    let messages = vec![Message {
        role: "assistant".into(),
        content: Content::Blocks(vec![
            ContentBlock::Thinking { thinking: "hmm".into() },
            ContentBlock::Unknown,
            ContentBlock::Text { text: "visible".into() },
        ]),
    }];
    let msgs = build_chat_completions_messages("", &messages);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content.as_deref(), Some("visible"));
    assert!(msgs[0].tool_calls.is_none());
}

// ===== build_responses_input =====

#[test]
fn build_resp_input_maps_text_messages() {
    let messages = vec![
        Message { role: "user".into(), content: Content::Text("hello".into()) },
        Message { role: "assistant".into(), content: Content::Text("hi".into()) },
    ];
    let items = build_responses_input(&messages);
    assert_eq!(items.len(), 2);
    let json = serde_json::to_value(&items[0]).unwrap();
    assert_eq!(json["type"], "message");
    assert_eq!(json["role"], "user");
    assert_eq!(json["content"][0]["text"], "hello");
}

#[test]
fn build_resp_input_maps_tool_use_to_function_call() {
    let messages = vec![Message {
        role: "assistant".into(),
        content: Content::Blocks(vec![ContentBlock::ToolUse {
            id: "call_1".into(),
            name: "test_fn".into(),
            input: serde_json::json!({"x": 1}),
        }]),
    }];
    let items = build_responses_input(&messages);
    assert_eq!(items.len(), 1);
    let json = serde_json::to_value(&items[0]).unwrap();
    assert_eq!(json["type"], "function_call");
    assert_eq!(json["call_id"], "call_1");
    assert_eq!(json["name"], "test_fn");
}

#[test]
fn build_resp_input_maps_tool_result_to_function_call_output() {
    let messages = vec![Message {
        role: "user".into(),
        content: Content::Blocks(vec![ContentBlock::ToolResult {
            tool_use_id: "call_1".into(),
            content: "output data".into(),
            is_error: None,
        }]),
    }];
    let items = build_responses_input(&messages);
    assert_eq!(items.len(), 1);
    let json = serde_json::to_value(&items[0]).unwrap();
    assert_eq!(json["type"], "function_call_output");
    assert_eq!(json["call_id"], "call_1");
    assert_eq!(json["output"], "output data");
}

#[test]
fn build_resp_input_skips_thinking_and_unknown() {
    let messages = vec![Message {
        role: "assistant".into(),
        content: Content::Blocks(vec![ContentBlock::Thinking { thinking: "hmm".into() }, ContentBlock::Unknown]),
    }];
    let items = build_responses_input(&messages);
    assert!(items.is_empty());
}

#[test]
fn build_resp_input_text_block_becomes_message() {
    let messages = vec![Message {
        role: "assistant".into(),
        content: Content::Blocks(vec![ContentBlock::Text { text: "response text".into() }]),
    }];
    let items = build_responses_input(&messages);
    assert_eq!(items.len(), 1);
    let json = serde_json::to_value(&items[0]).unwrap();
    assert_eq!(json["type"], "message");
    assert_eq!(json["content"][0]["text"], "response text");
}
