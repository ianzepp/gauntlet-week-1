//! LLM types — provider-neutral message types and errors.
//!
//! Ported from Prior's `kernel/src/llm/types.rs`. Provider-neutral types
//! shared by Anthropic and `OpenAI` clients.

use serde::{Deserialize, Serialize};

// =============================================================================
// ERROR
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("config parse failed: {0}")]
    ConfigParse(String),

    #[error("missing API key: env var {var} not set")]
    MissingApiKey { var: String },

    #[error("API request failed: {0}")]
    ApiRequest(String),

    #[error("API response error: status {status}")]
    ApiResponse { status: u16, body: String },

    #[error("API response parse failed: {0}")]
    ApiParse(String),

    #[error("HTTP client build failed: {0}")]
    HttpClientBuild(String),
}

impl crate::frame::ErrorCode for LlmError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::ConfigParse(_) => "E_CONFIG_PARSE",
            Self::MissingApiKey { .. } => "E_MISSING_API_KEY",
            Self::ApiRequest(_) => "E_API_REQUEST",
            Self::ApiResponse { .. } => "E_API_RESPONSE",
            Self::ApiParse(_) => "E_API_PARSE",
            Self::HttpClientBuild(_) => "E_HTTP_CLIENT_BUILD",
        }
    }

    fn retryable(&self) -> bool {
        matches!(self, Self::ApiRequest(_) | Self::ApiResponse { status: 429 | 500..=599, .. })
    }
}

// =============================================================================
// CONTENT BLOCKS
// =============================================================================

/// A structured content block in a message or API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },

    #[serde(rename = "thinking")]
    Thinking { thinking: String },

    #[serde(other)]
    Unknown,
}

/// Message content — either plain text or structured blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

// =============================================================================
// TOOL DEFINITION
// =============================================================================

/// A tool definition passed to the LLM provider API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

// =============================================================================
// MESSAGE TYPES
// =============================================================================

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Content,
}

/// Response from an LLM chat call.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

// =============================================================================
// LLM CHAT TRAIT
// =============================================================================

/// Provider-neutral async trait for LLM chat. Enables mocking in tests.
#[async_trait::async_trait]
pub trait LlmChat: Send + Sync {
    async fn chat(
        &self,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError>;
}
