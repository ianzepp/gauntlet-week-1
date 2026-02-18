use super::*;
use crate::frame::ErrorCode;

// =============================================================================
// LlmError::error_code — all 6 variants
// =============================================================================

#[test]
fn error_code_config_parse() {
    let err = LlmError::ConfigParse("bad".into());
    assert_eq!(err.error_code(), "E_CONFIG_PARSE");
}

#[test]
fn error_code_missing_api_key() {
    let err = LlmError::MissingApiKey { var: "KEY".into() };
    assert_eq!(err.error_code(), "E_MISSING_API_KEY");
}

#[test]
fn error_code_api_request() {
    let err = LlmError::ApiRequest("timeout".into());
    assert_eq!(err.error_code(), "E_API_REQUEST");
}

#[test]
fn error_code_api_response() {
    let err = LlmError::ApiResponse { status: 500, body: "oops".into() };
    assert_eq!(err.error_code(), "E_API_RESPONSE");
}

#[test]
fn error_code_api_parse() {
    let err = LlmError::ApiParse("json".into());
    assert_eq!(err.error_code(), "E_API_PARSE");
}

#[test]
fn error_code_http_client_build() {
    let err = LlmError::HttpClientBuild("tls".into());
    assert_eq!(err.error_code(), "E_HTTP_CLIENT_BUILD");
}

// =============================================================================
// LlmError::retryable
// =============================================================================

#[test]
fn retryable_api_request() {
    let err = LlmError::ApiRequest("conn refused".into());
    assert!(err.retryable());
}

#[test]
fn retryable_api_response_429() {
    let err = LlmError::ApiResponse { status: 429, body: "rate limited".into() };
    assert!(err.retryable());
}

#[test]
fn retryable_api_response_500() {
    let err = LlmError::ApiResponse { status: 500, body: "internal".into() };
    assert!(err.retryable());
}

#[test]
fn retryable_api_response_503() {
    let err = LlmError::ApiResponse { status: 503, body: "unavailable".into() };
    assert!(err.retryable());
}

#[test]
fn not_retryable_api_response_400() {
    let err = LlmError::ApiResponse { status: 400, body: "bad request".into() };
    assert!(!err.retryable());
}

#[test]
fn not_retryable_api_response_401() {
    let err = LlmError::ApiResponse { status: 401, body: "unauthorized".into() };
    assert!(!err.retryable());
}

#[test]
fn not_retryable_config_parse() {
    let err = LlmError::ConfigParse("bad".into());
    assert!(!err.retryable());
}

#[test]
fn not_retryable_missing_api_key() {
    let err = LlmError::MissingApiKey { var: "K".into() };
    assert!(!err.retryable());
}

#[test]
fn not_retryable_api_parse() {
    let err = LlmError::ApiParse("json".into());
    assert!(!err.retryable());
}

#[test]
fn not_retryable_http_client_build() {
    let err = LlmError::HttpClientBuild("tls".into());
    assert!(!err.retryable());
}

// =============================================================================
// LlmError Display
// =============================================================================

#[test]
fn display_config_parse() {
    let err = LlmError::ConfigParse("bad config".into());
    assert!(err.to_string().contains("bad config"));
}

#[test]
fn display_missing_api_key() {
    let err = LlmError::MissingApiKey { var: "MY_KEY".into() };
    let msg = err.to_string();
    assert!(msg.contains("MY_KEY"));
}

// =============================================================================
// ContentBlock serde round-trips
// =============================================================================

#[test]
fn content_block_text_round_trip() {
    let block = ContentBlock::Text { text: "hello".into() };
    let json = serde_json::to_string(&block).unwrap();
    let restored: ContentBlock = serde_json::from_str(&json).unwrap();
    match restored {
        ContentBlock::Text { text } => assert_eq!(text, "hello"),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn content_block_tool_use_round_trip() {
    let block = ContentBlock::ToolUse {
        id: "tu_1".into(),
        name: "createStickyNote".into(),
        input: serde_json::json!({"x": 10, "y": 20}),
    };
    let json = serde_json::to_string(&block).unwrap();
    let restored: ContentBlock = serde_json::from_str(&json).unwrap();
    match restored {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "tu_1");
            assert_eq!(name, "createStickyNote");
            assert_eq!(input["x"], 10);
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }
}

#[test]
fn content_block_tool_result_round_trip() {
    let block =
        ContentBlock::ToolResult { tool_use_id: "tu_1".into(), content: "success".into(), is_error: Some(false) };
    let json = serde_json::to_string(&block).unwrap();
    let restored: ContentBlock = serde_json::from_str(&json).unwrap();
    match restored {
        ContentBlock::ToolResult { tool_use_id, content, is_error } => {
            assert_eq!(tool_use_id, "tu_1");
            assert_eq!(content, "success");
            assert_eq!(is_error, Some(false));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

#[test]
fn content_block_tool_result_is_error_none_skipped() {
    let block = ContentBlock::ToolResult { tool_use_id: "tu_2".into(), content: "ok".into(), is_error: None };
    let json = serde_json::to_string(&block).unwrap();
    assert!(!json.contains("is_error"));
}

#[test]
fn content_block_thinking_round_trip() {
    let block = ContentBlock::Thinking { thinking: "hmm...".into() };
    let json = serde_json::to_string(&block).unwrap();
    let restored: ContentBlock = serde_json::from_str(&json).unwrap();
    match restored {
        ContentBlock::Thinking { thinking } => assert_eq!(thinking, "hmm..."),
        other => panic!("expected Thinking, got {other:?}"),
    }
}

#[test]
fn content_block_unknown_variant() {
    let json = r#"{"type": "some_future_type", "data": 123}"#;
    let block: ContentBlock = serde_json::from_str(json).unwrap();
    assert!(matches!(block, ContentBlock::Unknown));
}

// =============================================================================
// Content serde
// =============================================================================

#[test]
fn content_text_variant_round_trip() {
    let content = Content::Text("hello world".into());
    let json = serde_json::to_string(&content).unwrap();
    let restored: Content = serde_json::from_str(&json).unwrap();
    match restored {
        Content::Text(s) => assert_eq!(s, "hello world"),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn content_blocks_variant_round_trip() {
    let content = Content::Blocks(vec![ContentBlock::Text { text: "hi".into() }]);
    let json = serde_json::to_string(&content).unwrap();
    let restored: Content = serde_json::from_str(&json).unwrap();
    match restored {
        Content::Blocks(blocks) => {
            assert_eq!(blocks.len(), 1);
            match &blocks[0] {
                ContentBlock::Text { text } => assert_eq!(text, "hi"),
                other => panic!("expected Text block, got {other:?}"),
            }
        }
        other => panic!("expected Blocks, got {other:?}"),
    }
}

// =============================================================================
// Message serde
// =============================================================================

#[test]
fn message_text_round_trip() {
    let msg = Message { role: "user".into(), content: Content::Text("what is 2+2?".into()) };
    let json = serde_json::to_string(&msg).unwrap();
    let restored: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.role, "user");
    match restored.content {
        Content::Text(s) => assert_eq!(s, "what is 2+2?"),
        other => panic!("expected Text, got {other:?}"),
    }
}

// =============================================================================
// Tool serde
// =============================================================================

#[test]
fn tool_round_trip() {
    let tool = Tool {
        name: "createStickyNote".into(),
        description: "Creates a sticky note".into(),
        input_schema: serde_json::json!({"type": "object", "properties": {}}),
    };
    let json = serde_json::to_string(&tool).unwrap();
    let restored: Tool = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.name, "createStickyNote");
    assert_eq!(restored.description, "Creates a sticky note");
}

// =============================================================================
// ChatResponse serde
// =============================================================================

#[test]
fn chat_response_round_trip() {
    let resp = ChatResponse {
        content: vec![ContentBlock::Text { text: "Hello!".into() }],
        model: "claude-test".into(),
        stop_reason: "end_turn".into(),
        input_tokens: 100,
        output_tokens: 50,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let restored: ChatResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.model, "claude-test");
    assert_eq!(restored.stop_reason, "end_turn");
    assert_eq!(restored.input_tokens, 100);
    assert_eq!(restored.output_tokens, 50);
    assert_eq!(restored.content.len(), 1);
}

#[test]
fn chat_response_empty_content() {
    let resp = ChatResponse {
        content: vec![],
        model: "m".into(),
        stop_reason: "end_turn".into(),
        input_tokens: 0,
        output_tokens: 0,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let restored: ChatResponse = serde_json::from_str(&json).unwrap();
    assert!(restored.content.is_empty());
}
