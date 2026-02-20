//! Shared auth UI helpers.
//!
//! SYSTEM CONTEXT
//! ==============
//! Route components should apply identical unauthenticated redirect behavior.

use leptos::prelude::*;
use leptos_router::NavigateOptions;

use crate::state::auth::AuthState;

/// Redirect to `/login` whenever auth has loaded and no user is present.
pub fn install_unauth_redirect<F>(auth: RwSignal<AuthState>, navigate: F)
where
    F: Fn(&str, NavigateOptions) + Clone + 'static,
{
    let navigate = navigate.clone();
    Effect::new(move || {
        let state = auth.get();
        if !state.loading && state.user.is_none() {
            navigate("/login", NavigateOptions::default());
        }
    });
}
