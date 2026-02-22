//! LLM configuration parsed from environment variables.

use super::types::LlmError;

pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_LLM_REQUEST_TIMEOUT_SECS: u64 = 120;
pub const DEFAULT_LLM_CONNECT_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProviderKind {
    Anthropic,
    OpenAi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiApiMode {
    ChatCompletions,
    Responses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmTimeouts {
    pub request_secs: u64,
    pub connect_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmConfig {
    pub provider: LlmProviderKind,
    pub api_key: String,
    pub model: String,
    pub openai_mode: OpenAiApiMode,
    pub openai_base_url: String,
    pub timeouts: LlmTimeouts,
}

impl LlmConfig {
    /// Build typed LLM config from environment variables.
    ///
    /// Required:
    /// - `LLM_API_KEY_ENV` (names the env var containing the key)
    ///
    /// Optional:
    /// - `LLM_PROVIDER`: `anthropic` (default) or `openai`
    /// - `LLM_MODEL`: provider default when absent
    /// - `LLM_OPENAI_MODE`: `responses` (default) or `chat_completions`
    /// - `LLM_OPENAI_BASE_URL`: default OpenAI API base URL
    /// - `LLM_REQUEST_TIMEOUT_SECS`: default 120
    /// - `LLM_CONNECT_TIMEOUT_SECS`: default 10
    pub fn from_env() -> Result<Self, LlmError> {
        let provider = parse_provider(std::env::var("LLM_PROVIDER").ok().as_deref())?;

        let key_var =
            std::env::var("LLM_API_KEY_ENV").map_err(|_| LlmError::MissingApiKey { var: "LLM_API_KEY_ENV".into() })?;
        let api_key = std::env::var(&key_var).map_err(|_| LlmError::MissingApiKey { var: key_var.clone() })?;

        let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| default_model(provider).to_string());
        let openai_mode = parse_openai_mode(std::env::var("LLM_OPENAI_MODE").ok().as_deref())?;
        let openai_base_url = std::env::var("LLM_OPENAI_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        let timeouts = LlmTimeouts {
            request_secs: env_parse_u64("LLM_REQUEST_TIMEOUT_SECS", DEFAULT_LLM_REQUEST_TIMEOUT_SECS),
            connect_secs: env_parse_u64("LLM_CONNECT_TIMEOUT_SECS", DEFAULT_LLM_CONNECT_TIMEOUT_SECS),
        };

        Ok(Self { provider, api_key, model, openai_mode, openai_base_url, timeouts })
    }
}

fn env_parse_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_provider(raw: Option<&str>) -> Result<LlmProviderKind, LlmError> {
    match raw.unwrap_or("anthropic") {
        "anthropic" => Ok(LlmProviderKind::Anthropic),
        "openai" => Ok(LlmProviderKind::OpenAi),
        other => Err(LlmError::ConfigParse(format!("unknown LLM_PROVIDER: {other}"))),
    }
}

fn parse_openai_mode(raw: Option<&str>) -> Result<OpenAiApiMode, LlmError> {
    match raw.unwrap_or("responses") {
        "responses" => Ok(OpenAiApiMode::Responses),
        "chat_completions" => Ok(OpenAiApiMode::ChatCompletions),
        other => Err(LlmError::ConfigParse(format!(
            "unsupported openai_api mode '{other}' (expected 'responses' or 'chat_completions')"
        ))),
    }
}

fn default_model(provider: LlmProviderKind) -> &'static str {
    match provider {
        LlmProviderKind::Anthropic => "claude-sonnet-4-5-20250929",
        LlmProviderKind::OpenAi => "gpt-4o",
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
