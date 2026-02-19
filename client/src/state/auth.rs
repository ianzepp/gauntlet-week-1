//! Auth-session state for the current browser user.
//!
//! SYSTEM CONTEXT
//! ==============
//! Used by route guards and user-aware components to coordinate login redirects
//! and identity-dependent rendering.

#[cfg(test)]
#[path = "auth_test.rs"]
mod auth_test;

use crate::net::types::User;

/// Authentication state tracking the current user and loading status.
///
/// In the full Leptos implementation, fields will be `RwSignal` types
/// provided via context. For now they are plain fields.
#[derive(Clone, Debug, Default)]
pub struct AuthState {
    pub user: Option<User>,
    pub loading: bool,
}
