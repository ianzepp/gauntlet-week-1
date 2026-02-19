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

use super::types::{User, UserProfile};

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
        let url = format!("/api/users/{user_id}/profile");
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
            return Err(format!("ticket request failed: {}", resp.status()));
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
