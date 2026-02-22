use super::*;

/// # Safety
/// Tests must run with `--test-threads=1` to avoid env races.
unsafe fn clear_llm_env() {
    unsafe {
        std::env::remove_var("LLM_PROVIDER");
        std::env::remove_var("LLM_MODEL");
        std::env::remove_var("LLM_API_KEY_ENV");
        std::env::remove_var("LLM_OPENAI_MODE");
        std::env::remove_var("LLM_OPENAI_BASE_URL");
        std::env::remove_var("LLM_REQUEST_TIMEOUT_SECS");
        std::env::remove_var("LLM_CONNECT_TIMEOUT_SECS");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("TEST_KEY");
    }
}

#[test]
fn from_env_defaults_to_anthropic() {
    unsafe {
        clear_llm_env();
        std::env::set_var("LLM_API_KEY_ENV", "TEST_KEY");
        std::env::set_var("TEST_KEY", "secret");
    }

    let cfg = LlmConfig::from_env().unwrap();
    assert_eq!(cfg.provider, LlmProviderKind::Anthropic);
    assert_eq!(cfg.model, "claude-sonnet-4-5-20250929");
    assert_eq!(cfg.openai_mode, OpenAiApiMode::Responses);
    assert_eq!(cfg.openai_base_url, DEFAULT_OPENAI_BASE_URL);
    assert_eq!(
        cfg.timeouts,
        LlmTimeouts { request_secs: DEFAULT_LLM_REQUEST_TIMEOUT_SECS, connect_secs: DEFAULT_LLM_CONNECT_TIMEOUT_SECS }
    );
    assert_eq!(cfg.api_key, "secret");

    unsafe { clear_llm_env() };
}

#[test]
fn from_env_parses_openai_overrides() {
    unsafe {
        clear_llm_env();
        std::env::set_var("LLM_PROVIDER", "openai");
        std::env::set_var("LLM_API_KEY_ENV", "OPENAI_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "sk-test");
        std::env::set_var("LLM_OPENAI_MODE", "chat_completions");
        std::env::set_var("LLM_OPENAI_BASE_URL", "https://example.test/v1/");
        std::env::set_var("LLM_REQUEST_TIMEOUT_SECS", "42");
        std::env::set_var("LLM_CONNECT_TIMEOUT_SECS", "7");
    }

    let cfg = LlmConfig::from_env().unwrap();
    assert_eq!(cfg.provider, LlmProviderKind::OpenAi);
    assert_eq!(cfg.model, "gpt-4o");
    assert_eq!(cfg.openai_mode, OpenAiApiMode::ChatCompletions);
    assert_eq!(cfg.openai_base_url, "https://example.test/v1");
    assert_eq!(cfg.timeouts, LlmTimeouts { request_secs: 42, connect_secs: 7 });

    unsafe { clear_llm_env() };
}

#[test]
fn from_env_unknown_provider_errors() {
    unsafe {
        clear_llm_env();
        std::env::set_var("LLM_PROVIDER", "bad");
        std::env::set_var("LLM_API_KEY_ENV", "TEST_KEY");
        std::env::set_var("TEST_KEY", "secret");
    }

    let err = LlmConfig::from_env().unwrap_err().to_string();
    assert!(err.contains("unknown LLM_PROVIDER"));

    unsafe { clear_llm_env() };
}

#[test]
fn from_env_unknown_openai_mode_errors() {
    unsafe {
        clear_llm_env();
        std::env::set_var("LLM_API_KEY_ENV", "TEST_KEY");
        std::env::set_var("TEST_KEY", "secret");
        std::env::set_var("LLM_OPENAI_MODE", "bad_mode");
    }

    let err = LlmConfig::from_env().unwrap_err().to_string();
    assert!(err.contains("unsupported openai_api mode"));

    unsafe { clear_llm_env() };
}
