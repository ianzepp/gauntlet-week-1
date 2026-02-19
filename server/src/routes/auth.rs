//! Auth routes — GitHub OAuth flow, session management, WS tickets.

use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use time::Duration;
use uuid::Uuid;

use crate::services::{auth as auth_svc, session};
use crate::state::AppState;

const COOKIE_NAME: &str = "session_token";
const OAUTH_STATE_COOKIE_NAME: &str = "oauth_state";

pub(crate) fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key)
        .ok()
        .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
}

pub(crate) fn cookie_secure() -> bool {
    if let Some(value) = env_bool("COOKIE_SECURE") {
        return value;
    }

    std::env::var("GITHUB_REDIRECT_URI")
        .map(|uri| uri.starts_with("https://"))
        .unwrap_or(false)
}

fn perf_test_auth_bypass_enabled() -> bool {
    env_bool("PERF_TEST_AUTH_BYPASS").unwrap_or(false)
}

// =============================================================================
// AUTH EXTRACTOR
// =============================================================================

/// Authenticated user extracted from the session cookie.
/// Use as a handler parameter to require authentication.
pub struct AuthUser {
    pub user: session::SessionUser,
    pub token: String,
}

impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(COOKIE_NAME).map(Cookie::value).unwrap_or_default();
        if token.is_empty() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let app_state = AppState::from_ref(state);
        let user = session::validate_session(&app_state.pool, token)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(Self { user, token: token.to_owned() })
    }
}

// =============================================================================
// HANDLERS
// =============================================================================

/// `GET /auth/github` — redirect to GitHub authorization page.
pub async fn github_redirect(State(state): State<AppState>) -> Response {
    let Some(config) = &state.github else {
        return (StatusCode::SERVICE_UNAVAILABLE, "GitHub OAuth not configured").into_response();
    };

    let oauth_state = session::generate_token();
    let secure = cookie_secure();
    let cookie = Cookie::build((OAUTH_STATE_COOKIE_NAME, oauth_state.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(secure)
        .max_age(Duration::minutes(10));

    let jar = CookieJar::new().add(cookie);
    (jar, Redirect::temporary(&config.authorize_url(&oauth_state))).into_response()
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: Option<String>,
}

/// `GET /auth/github/callback` — exchange code, upsert user, set cookie, redirect to `/`.
pub async fn github_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<CallbackQuery>,
) -> Response {
    let Some(config) = &state.github else {
        return (StatusCode::SERVICE_UNAVAILABLE, "GitHub OAuth not configured").into_response();
    };
    let secure = cookie_secure();

    // Verify OAuth CSRF state from cookie.
    let Some(callback_state) = params.state.as_deref() else {
        return (StatusCode::BAD_REQUEST, "missing oauth state").into_response();
    };
    let expected_state = jar
        .get(OAUTH_STATE_COOKIE_NAME)
        .map(Cookie::value)
        .unwrap_or_default();
    if expected_state.is_empty() || expected_state != callback_state {
        return (StatusCode::UNAUTHORIZED, "invalid oauth state").into_response();
    }

    // Exchange code for access token.
    let access_token = match auth_svc::exchange_code(config, &params.code).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "oauth code exchange failed");
            return (StatusCode::BAD_GATEWAY, "OAuth code exchange failed").into_response();
        }
    };

    // Fetch GitHub user profile.
    let gh_user = match auth_svc::fetch_github_user(&access_token).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(error = %e, "github user fetch failed");
            return (StatusCode::BAD_GATEWAY, "Failed to fetch GitHub profile").into_response();
        }
    };

    // Upsert user in DB.
    let user_id = match auth_svc::upsert_user(&state.pool, &gh_user).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "user upsert failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response();
        }
    };

    // Create session.
    let token = match session::create_session(&state.pool, user_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "session creation failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session").into_response();
        }
    };

    // Set HttpOnly cookie and redirect to SPA.
    let session_cookie = Cookie::build((COOKIE_NAME, token))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(secure);
    let clear_oauth_state_cookie = Cookie::build((OAUTH_STATE_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(secure)
        .max_age(Duration::ZERO);

    let jar = jar.add(session_cookie).add(clear_oauth_state_cookie);
    (jar, Redirect::temporary("/")).into_response()
}

/// `GET /api/auth/me` — return current user.
pub async fn me(auth: AuthUser) -> Json<session::SessionUser> {
    Json(auth.user)
}

/// `POST /api/auth/logout` — delete session, clear cookie.
pub async fn logout(State(state): State<AppState>, auth: AuthUser) -> impl IntoResponse {
    let _ = session::delete_session(&state.pool, &auth.token).await;

    let secure = cookie_secure();
    let cookie = Cookie::build((COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(secure)
        .max_age(Duration::ZERO);

    let jar = CookieJar::new().add(cookie);
    (jar, StatusCode::NO_CONTENT)
}

/// `POST /api/auth/ws-ticket` — create a one-time WS ticket.
pub async fn ws_ticket(State(state): State<AppState>, auth: AuthUser) -> Result<Json<serde_json::Value>, StatusCode> {
    let ticket = session::create_ws_ticket(&state.pool, auth.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "ticket": ticket })))
}

/// `POST /api/dev/ws-ticket` — perf-test-only ticket bootstrap without OAuth/session.
///
/// Enabled only when `PERF_TEST_AUTH_BYPASS=true`.
pub async fn dev_ws_ticket(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !perf_test_auth_bypass_enabled() {
        return Err(StatusCode::NOT_FOUND);
    }

    let user_id =
        Uuid::parse_str("00000000-0000-0000-0000-00000000f00d").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_name = "Perf User".to_owned();

    sqlx::query(
        "INSERT INTO users (id, name, color) VALUES ($1, $2, $3)
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, color = EXCLUDED.color",
    )
    .bind(user_id)
    .bind(user_name)
    .bind("#4CAF50")
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ticket = session::create_ws_ticket(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "ticket": ticket })))
}

#[cfg(test)]
#[path = "auth_test.rs"]
mod tests;
