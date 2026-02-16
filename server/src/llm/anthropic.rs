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

fn parse_response(json: &str) -> Result<ChatResponse, LlmError> {
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
