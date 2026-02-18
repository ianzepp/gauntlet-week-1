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
