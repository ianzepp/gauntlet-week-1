//! LLM — multi-provider adapter for AI agent features.
//!
//! DESIGN
//! ======
//! Ported from Prior's `kernel/src/llm/mod.rs`. Simplified: uses environment
//! variables instead of config files. The `LlmClient` enum dispatches to
//! Anthropic or `OpenAI` based on `LLM_PROVIDER`.

pub mod anthropic;
pub mod config;
pub mod openai;
pub mod tools;
pub mod types;

use config::{LlmConfig, LlmProviderKind};
pub use types::LlmChat;
use types::{ChatResponse, LlmError, Message, Tool};

// =============================================================================
// CLIENT DISPATCH
// =============================================================================

/// Concrete LLM client that dispatches to either Anthropic or OpenAI.
///
/// Configured from environment variables by [`LlmClient::from_env`].
pub struct LlmClient {
    inner: LlmProvider,
    model: String,
}

enum LlmProvider {
    Anthropic(anthropic::AnthropicClient),
    OpenAi(openai::OpenAiClient),
}

impl LlmClient {
    /// Build an LLM client from environment variables.
    ///
    /// - `LLM_PROVIDER`: "anthropic" (default) or "openai"
    /// - `LLM_API_KEY_ENV`: name of env var holding the API key (e.g. `ANTHROPIC_API_KEY`)
    /// - `LLM_MODEL`: model name (e.g. "claude-sonnet-4-5-20250929")
    /// - `LLM_OPENAI_MODE`: "responses" (default) or `"chat_completions"`
    /// - `LLM_OPENAI_BASE_URL`: custom base URL for OpenAI-compatible APIs
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is missing or the HTTP client fails.
    pub fn from_env() -> Result<Self, LlmError> {
        let config = LlmConfig::from_env()?;
        Self::from_config(config)
    }

    /// Build an LLM client from a parsed typed config.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider HTTP client fails to build.
    pub fn from_config(config: LlmConfig) -> Result<Self, LlmError> {
        let model = config.model.clone();
        let inner = match config.provider {
            LlmProviderKind::Anthropic => {
                LlmProvider::Anthropic(anthropic::AnthropicClient::new(config.api_key, config.timeouts)?)
            }
            LlmProviderKind::OpenAi => LlmProvider::OpenAi(openai::OpenAiClient::new(
                config.api_key,
                config.openai_mode,
                config.openai_base_url,
                config.timeouts,
            )?),
        };
        Ok(Self { inner, model })
    }

    /// Return the configured model name (e.g. `"claude-sonnet-4-5-20250929"`).
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    async fn chat_inner(
        &self,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        match &self.inner {
            LlmProvider::Anthropic(c) => {
                c.chat(&self.model, max_tokens, system, messages, tools)
                    .await
            }
            LlmProvider::OpenAi(c) => {
                c.chat(&self.model, max_tokens, system, messages, tools)
                    .await
            }
        }
    }
}

#[async_trait::async_trait]
impl LlmChat for LlmClient {
    async fn chat(
        &self,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        self.chat_inner(max_tokens, system, messages, tools).await
    }
}
