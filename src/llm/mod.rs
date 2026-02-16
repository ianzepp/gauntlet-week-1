//! LLM — multi-provider adapter for AI agent features.
//!
//! DESIGN
//! ======
//! Ported from Prior's `kernel/src/llm/mod.rs`. Simplified: uses environment
//! variables instead of config files. The `LlmClient` enum dispatches to
//! Anthropic or OpenAI based on `LLM_PROVIDER`.

pub mod anthropic;
pub mod openai;
pub mod tools;
pub mod types;

use types::{ChatResponse, LlmError, Message, Tool};

// =============================================================================
// CLIENT DISPATCH
// =============================================================================

pub enum LlmClient {
    Anthropic(anthropic::AnthropicClient),
    OpenAi(openai::OpenAiClient),
}

impl LlmClient {
    /// Build an LLM client from environment variables.
    ///
    /// - `LLM_PROVIDER`: "anthropic" (default) or "openai"
    /// - `LLM_API_KEY`: provider API key
    /// - `LLM_OPENAI_MODE`: "responses" (default) or "chat_completions"
    /// - `LLM_OPENAI_BASE_URL`: custom base URL for OpenAI-compatible APIs
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing or the HTTP client fails.
    pub fn from_env() -> Result<Self, LlmError> {
        let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());
        let api_key =
            std::env::var("LLM_API_KEY").map_err(|_| LlmError::MissingApiKey { var: "LLM_API_KEY".into() })?;

        match provider.as_str() {
            "anthropic" => Ok(Self::Anthropic(anthropic::AnthropicClient::new(api_key)?)),
            "openai" => {
                let mode = std::env::var("LLM_OPENAI_MODE").ok();
                let base_url = std::env::var("LLM_OPENAI_BASE_URL").ok();
                Ok(Self::OpenAi(openai::OpenAiClient::new(
                    api_key,
                    mode.as_deref(),
                    base_url.as_deref(),
                )?))
            }
            other => Err(LlmError::ConfigParse(format!("unknown LLM_PROVIDER: {other}"))),
        }
    }

    pub async fn chat(
        &self,
        model: &str,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        match self {
            Self::Anthropic(c) => c.chat(model, max_tokens, system, messages, tools).await,
            Self::OpenAi(c) => c.chat(model, max_tokens, system, messages, tools).await,
        }
    }
}
