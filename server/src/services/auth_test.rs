use super::*;

// =============================================================================
// GitHubConfig::from_env — env manipulation requires unsafe in edition 2024.
// We wrap in unsafe blocks; these tests run serially (single test thread).
// =============================================================================

/// # Safety
/// Tests must run with `--test-threads=1` to avoid env races.
unsafe fn clear_github_env() {
    unsafe {
        std::env::remove_var("GITHUB_CLIENT_ID");
        std::env::remove_var("GITHUB_CLIENT_SECRET");
        std::env::remove_var("GITHUB_REDIRECT_URI");
    }
}

#[test]
fn from_env_all_set_returns_some() {
    unsafe {
        clear_github_env();
        std::env::set_var("GITHUB_CLIENT_ID", "id123");
        std::env::set_var("GITHUB_CLIENT_SECRET", "secret456");
        std::env::set_var("GITHUB_REDIRECT_URI", "http://localhost/callback");
    }
    let config = GitHubConfig::from_env();
    assert!(config.is_some());
    let config = config.unwrap();
    assert_eq!(config.client_id, "id123");
    assert_eq!(config.client_secret, "secret456");
    assert_eq!(config.redirect_uri, "http://localhost/callback");
    unsafe { clear_github_env() };
}

#[test]
fn from_env_missing_client_id_returns_none() {
    unsafe {
        clear_github_env();
        std::env::set_var("GITHUB_CLIENT_SECRET", "secret456");
        std::env::set_var("GITHUB_REDIRECT_URI", "http://localhost/callback");
    }
    assert!(GitHubConfig::from_env().is_none());
    unsafe { clear_github_env() };
}

#[test]
fn from_env_missing_secret_returns_none() {
    unsafe {
        clear_github_env();
        std::env::set_var("GITHUB_CLIENT_ID", "id123");
        std::env::set_var("GITHUB_REDIRECT_URI", "http://localhost/callback");
    }
    assert!(GitHubConfig::from_env().is_none());
    unsafe { clear_github_env() };
}

#[test]
fn from_env_missing_redirect_returns_none() {
    unsafe {
        clear_github_env();
        std::env::set_var("GITHUB_CLIENT_ID", "id123");
        std::env::set_var("GITHUB_CLIENT_SECRET", "secret456");
    }
    assert!(GitHubConfig::from_env().is_none());
    unsafe { clear_github_env() };
}

#[test]
fn from_env_all_missing_returns_none() {
    unsafe { clear_github_env() };
    assert!(GitHubConfig::from_env().is_none());
}

// =============================================================================
// authorize_url
// =============================================================================

#[test]
fn authorize_url_contains_client_id() {
    let config = GitHubConfig {
        client_id: "my_client_id".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost/cb".into(),
    };
    let url = config.authorize_url("random_state");
    assert!(url.contains("client_id=my_client_id"));
}

#[test]
fn authorize_url_contains_redirect_uri() {
    let config = GitHubConfig {
        client_id: "cid".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost/cb".into(),
    };
    let url = config.authorize_url("st");
    assert!(url.contains("redirect_uri="));
    assert!(url.contains("localhost"));
}

#[test]
fn authorize_url_contains_state_param() {
    let config = GitHubConfig {
        client_id: "cid".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost/cb".into(),
    };
    let url = config.authorize_url("csrf_token_abc");
    assert!(url.contains("state=csrf_token_abc"));
}

#[test]
fn authorize_url_contains_scope() {
    let config = GitHubConfig {
        client_id: "cid".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost/cb".into(),
    };
    let url = config.authorize_url("st");
    assert!(url.contains("scope=read"));
}

#[test]
fn authorize_url_starts_with_github() {
    let config = GitHubConfig {
        client_id: "cid".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost/cb".into(),
    };
    let url = config.authorize_url("st");
    assert!(url.starts_with("https://github.com/login/oauth/authorize"));
}

// =============================================================================
// AuthError display
// =============================================================================

#[test]
fn auth_error_token_exchange_display() {
    let err = AuthError::TokenExchange("timeout".into());
    let msg = err.to_string();
    assert!(msg.contains("token exchange"));
    assert!(msg.contains("timeout"));
}

#[test]
fn auth_error_github_api_display() {
    let err = AuthError::GitHubApi("403 Forbidden".into());
    let msg = err.to_string();
    assert!(msg.contains("github api"));
    assert!(msg.contains("403 Forbidden"));
}

// =============================================================================
// GitHubUser serde
// =============================================================================

#[test]
fn github_user_deserialize_with_avatar() {
    let json = r#"{"id": 12345, "login": "octocat", "avatar_url": "https://avatars.example.com/1"}"#;
    let user: GitHubUser = serde_json::from_str(json).unwrap();
    assert_eq!(user.id, 12345);
    assert_eq!(user.login, "octocat");
    assert_eq!(user.avatar_url.as_deref(), Some("https://avatars.example.com/1"));
}

#[test]
fn github_user_deserialize_without_avatar() {
    let json = r#"{"id": 67890, "login": "ghostuser", "avatar_url": null}"#;
    let user: GitHubUser = serde_json::from_str(json).unwrap();
    assert_eq!(user.id, 67890);
    assert_eq!(user.login, "ghostuser");
    assert!(user.avatar_url.is_none());
}

#[test]
fn github_config_debug() {
    let config = GitHubConfig {
        client_id: "cid".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost/cb".into(),
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("cid"));
}
