//! LLM — multi-provider adapter for AI agent features.
//!
//! DESIGN
//! ======
//! Ported from Prior's `kernel/src/llm/mod.rs`. Simplified: uses environment
//! variables instead of config files. The `LlmClient` enum dispatches to
//! Anthropic or `OpenAI` based on `LLM_PROVIDER`.

pub mod anthropic;
pub mod openai;
pub mod tools;
pub mod types;

pub use types::LlmChat;
use types::{ChatResponse, LlmError, Message, Tool};

// =============================================================================
// CLIENT DISPATCH
// =============================================================================

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
        let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());

        // Indirection: LLM_API_KEY_ENV names the env var that holds the actual key.
        let key_var =
            std::env::var("LLM_API_KEY_ENV").map_err(|_| LlmError::MissingApiKey { var: "LLM_API_KEY_ENV".into() })?;
        let api_key = std::env::var(&key_var).map_err(|_| LlmError::MissingApiKey { var: key_var.clone() })?;

        let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
            "openai" => "gpt-4o".into(),
            _ => "claude-sonnet-4-5-20250929".into(),
        });

        let inner = match provider.as_str() {
            "anthropic" => LlmProvider::Anthropic(anthropic::AnthropicClient::new(api_key)?),
            "openai" => {
                let mode = std::env::var("LLM_OPENAI_MODE").ok();
                let base_url = std::env::var("LLM_OPENAI_BASE_URL").ok();
                LlmProvider::OpenAi(openai::OpenAiClient::new(api_key, mode.as_deref(), base_url.as_deref())?)
            }
            other => return Err(LlmError::ConfigParse(format!("unknown LLM_PROVIDER: {other}"))),
        };

        Ok(Self { inner, model })
    }

    /// The configured model name.
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
