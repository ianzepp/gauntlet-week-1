//! REST API helpers for communicating with the server.
//!
//! Client-side (hydrate): real HTTP calls via `gloo-net`.
//! Server-side (SSR): stubs returning `None`/error since these endpoints
//! are only meaningful in the browser.
//!
//! ERROR HANDLING
//! ==============
//! Callers get `Option`/`Result` outputs instead of panics so auth/profile
//! fetch failures degrade UI behavior without crashing hydration.

#![allow(clippy::unused_async)]

#[cfg(test)]
#[path = "api_test.rs"]
mod api_test;

use super::types::{User, UserProfile};
#[cfg(feature = "hydrate")]
use serde::Deserialize;

#[cfg(any(test, feature = "hydrate"))]
fn user_profile_endpoint(user_id: &str) -> String {
    format!("/api/users/{user_id}/profile")
}

#[cfg(any(test, feature = "hydrate"))]
fn ticket_request_failed_message(status: u16) -> String {
    format!("ticket request failed: {status}")
}

#[cfg(any(test, feature = "hydrate"))]
fn request_code_failed_message(status: u16) -> String {
    format!("request code failed: {status}")
}

#[cfg(any(test, feature = "hydrate"))]
fn verify_code_failed_message(status: u16) -> String {
    format!("verify code failed: {status}")
}

/// Fetch the currently authenticated user from `/api/auth/me`.
/// Returns `None` if not authenticated or on the server.
pub async fn fetch_current_user() -> Option<User> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/auth/me")
            .send()
            .await
            .ok()?;
        if !resp.ok() {
            return None;
        }
        resp.json::<User>().await.ok()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        None
    }
}

/// Log out the current user by calling `POST /api/auth/logout`.
pub async fn logout() {
    #[cfg(feature = "hydrate")]
    {
        let _ = gloo_net::http::Request::post("/api/auth/logout")
            .send()
            .await;
    }
}

/// Fetch a user's profile from `/api/users/{user_id}/profile`.
pub async fn fetch_user_profile(user_id: &str) -> Option<UserProfile> {
    #[cfg(feature = "hydrate")]
    {
        let url = user_profile_endpoint(user_id);
        let resp = gloo_net::http::Request::get(&url).send().await.ok()?;
        if !resp.ok() {
            return None;
        }
        resp.json::<UserProfile>().await.ok()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = user_id;
        None
    }
}

/// Create a WebSocket authentication ticket via `POST /api/auth/ws-ticket`.
///
/// # Errors
///
/// Returns an error string if the ticket request fails.
pub async fn create_ws_ticket() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/auth/ws-ticket")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(ticket_request_failed_message(resp.status()));
        }
        #[derive(serde::Deserialize)]
        struct TicketResponse {
            ticket: String,
        }
        let body: TicketResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body.ticket)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Err("not available on server".to_owned())
    }
}

#[cfg(feature = "hydrate")]
#[derive(Debug, Deserialize)]
struct RequestEmailCodeResponse {
    ok: bool,
    code: Option<String>,
}

/// Request a 6-character email login code via `POST /api/auth/email/request-code`.
///
/// Returns an optional code string when the server is configured to echo codes.
///
/// # Errors
///
/// Returns an error string if the HTTP request fails or the server responds with a non-OK status.
pub async fn request_email_login_code(email: &str) -> Result<Option<String>, String> {
    #[cfg(feature = "hydrate")]
    {
        let payload = serde_json::json!({ "email": email });
        let resp = gloo_net::http::Request::post("/api/auth/email/request-code")
            .json(&payload)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(request_code_failed_message(resp.status()));
        }
        let body: RequestEmailCodeResponse = resp.json().await.map_err(|e| e.to_string())?;
        if !body.ok {
            return Err("request code failed".to_owned());
        }
        Ok(body.code)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = email;
        Err("not available on server".to_owned())
    }
}

#[cfg(feature = "hydrate")]
#[derive(Debug, Deserialize)]
struct VerifyEmailCodeResponse {
    ok: bool,
}

/// Verify an email login code via `POST /api/auth/email/verify-code`.
///
/// # Errors
///
/// Returns an error string if the HTTP request fails, the server responds with a non-OK status,
/// or the verification code is rejected.
pub async fn verify_email_login_code(email: &str, code: &str) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let payload = serde_json::json!({ "email": email, "code": code });
        let resp = gloo_net::http::Request::post("/api/auth/email/verify-code")
            .json(&payload)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(verify_code_failed_message(resp.status()));
        }
        let body: VerifyEmailCodeResponse = resp.json().await.map_err(|e| e.to_string())?;
        if !body.ok {
            return Err("verify code failed".to_owned());
        }
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (email, code);
        Err("not available on server".to_owned())
    }
}
