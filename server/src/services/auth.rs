//! GitHub OAuth service — code exchange, profile fetch, user upsert.
//!
//! SYSTEM CONTEXT
//! ==============
//! Route handlers call this module to keep OAuth transport details isolated
//! from session and websocket logic.
//!
//! ERROR HANDLING
//! ==============
//! Upstream HTTP failures are normalized into `AuthError` variants so callers
//! can translate them into stable API error responses.

use sqlx::{PgPool, Row};
use uuid::Uuid;

/// GitHub OAuth configuration loaded from environment.
#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl GitHubConfig {
    /// Load from `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_REDIRECT_URI`.
    /// Returns `None` if any are missing (auth will be disabled).
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("GITHUB_CLIENT_ID").ok()?;
        let client_secret = std::env::var("GITHUB_CLIENT_SECRET").ok()?;
        let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").ok()?;
        Some(Self { client_id, client_secret, redirect_uri })
    }

    /// Build the GitHub authorization URL.
    #[must_use]
    pub fn authorize_url(&self, state: &str) -> String {
        // The base URL is a known-valid constant; treat a parse failure as a
        // configuration bug and return a best-effort fallback so callers get a
        // predictable string type rather than a panic.
        let Ok(mut url) = reqwest::Url::parse("https://github.com/login/oauth/authorize") else {
            return String::from("https://github.com/login/oauth/authorize");
        };
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", "read:user")
            .append_pair("state", state);
        url.to_string()
    }
}

#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("github token exchange failed: {0}")]
    TokenExchange(String),
    #[error("github api error: {0}")]
    GitHubApi(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

/// Exchange an OAuth code for an access token.
pub async fn exchange_code(config: &GitHubConfig, code: &str) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": config.client_id,
            "client_secret": config.client_secret,
            "code": code,
            "redirect_uri": config.redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| AuthError::TokenExchange(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::TokenExchange(format!("{status}: {body}")));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| AuthError::TokenExchange(e.to_string()))?;
    let token_resp: TokenResponse =
        serde_json::from_str(&body).map_err(|_| AuthError::TokenExchange(format!("unexpected response: {body}")))?;
    Ok(token_resp.access_token)
}

/// Fetch the authenticated GitHub user's profile.
pub async fn fetch_github_user(access_token: &str) -> Result<GitHubUser, AuthError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", "server")
        .send()
        .await
        .map_err(|e| AuthError::GitHubApi(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::GitHubApi(format!("{status}: {body}")));
    }

    resp.json::<GitHubUser>()
        .await
        .map_err(|e| AuthError::GitHubApi(e.to_string()))
}

/// Upsert a user from their GitHub profile. Returns the user's UUID.
pub async fn upsert_user(pool: &PgPool, gh: &GitHubUser) -> Result<Uuid, AuthError> {
    let row = sqlx::query(
        r"INSERT INTO users (github_id, name, avatar_url)
          VALUES ($1, $2, $3)
          ON CONFLICT (github_id) DO UPDATE SET name = EXCLUDED.name, avatar_url = EXCLUDED.avatar_url
          RETURNING id",
    )
    .bind(gh.id)
    .bind(&gh.login)
    .bind(&gh.avatar_url)
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

#[cfg(test)]
#[path = "auth_test.rs"]
mod tests;
