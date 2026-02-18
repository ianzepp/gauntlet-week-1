//! REST API helpers for communicating with the server.
//!
//! These functions wrap HTTP calls to the server's REST endpoints.
//! They require a browser environment (WASM) and will be implemented
//! using `gloo-net` once the WASM target is configured.
//!
//! All functions are currently stubs returning default/error values.
//! The `async` and `unused` lints are suppressed at the module level
//! since these will become real async calls once gloo-net is added.

#![allow(clippy::unused_async)]

use super::types::{User, UserProfile};

/// Fetch the currently authenticated user from `/api/auth/me`.
/// Returns `None` if not authenticated.
pub async fn fetch_current_user() -> Option<User> {
    // Requires gloo-net / WASM target
    None
}

/// Log out the current user by calling `/api/auth/logout`.
pub async fn logout() {
    // Requires gloo-net / WASM target
}

/// Fetch a user's profile from `/api/users/{user_id}/profile`.
pub async fn fetch_user_profile(_user_id: &str) -> Option<UserProfile> {
    // Requires gloo-net / WASM target
    None
}

/// Create a WebSocket authentication ticket via `/api/ws/ticket`.
///
/// # Errors
///
/// Returns an error string if the ticket request fails.
pub async fn create_ws_ticket() -> Result<String, String> {
    // Requires gloo-net / WASM target
    Err("not yet implemented".to_owned())
}
