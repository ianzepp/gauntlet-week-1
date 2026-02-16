//! Anthropic Messages API client.
//!
//! Ported from Prior's `kernel/src/llm/client.rs`. Thin HTTP wrapper for
//! `/v1/messages`. Pure parsing in `parse_response` for testability.

use super::types::{ChatResponse, ContentBlock, LlmError, Message, Tool};
use std::time::Duration;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const REQUEST_TIMEOUT_SECS: u64 = 120;
const CONNECT_TIMEOUT_SECS: u64 = 10;

// =============================================================================
// CLIENT
// =============================================================================

pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .build()
            .map_err(|e| LlmError::HttpClientBuild(e.to_string()))?;
        Ok(Self { http, api_key })
    }

    pub async fn chat(
        &self,
        model: &str,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        let body = ApiRequest { model, max_tokens, system, messages, tools };

        let response = self
            .http
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::ApiRequest(e.to_string()))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| LlmError::ApiRequest(e.to_string()))?;

        if status != 200 {
            return Err(LlmError::ApiResponse { status, body: text });
        }

        parse_response(&text)
    }
}

// =============================================================================
// WIRE TYPES
// =============================================================================

#[derive(serde::Serialize)]
struct ApiRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: &'a [Message],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [Tool]>,
}

#[derive(serde::Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: String,
    usage: Usage,
}

#[derive(serde::Deserialize)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
}

// =============================================================================
// PARSING
// =============================================================================

pub(crate) fn parse_response(json: &str) -> Result<ChatResponse, LlmError> {
    let api: ApiResponse = serde_json::from_str(json).map_err(|e| LlmError::ApiParse(e.to_string()))?;

    let content: Vec<ContentBlock> = api
        .content
        .into_iter()
        .filter(|block| !matches!(block, ContentBlock::Unknown))
        .collect();

    Ok(ChatResponse {
        content,
        model: api.model,
        stop_reason: api.stop_reason,
        input_tokens: api.usage.input_tokens,
        output_tokens: api.usage.output_tokens,
    })
}

#[cfg(test)]
mod tests {
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
}
