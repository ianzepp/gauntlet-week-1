//! Shared auth UI helpers.
//!
//! SYSTEM CONTEXT
//! ==============
//! Route components should apply identical unauthenticated redirect behavior.

#[cfg(test)]
#[path = "auth_test.rs"]
mod auth_test;

use leptos::prelude::*;
use leptos_router::NavigateOptions;

use crate::state::auth::AuthState;

fn should_redirect_unauth(state: &AuthState) -> bool {
    !state.loading && state.user.is_none()
}

/// Redirect to `/login` whenever auth has loaded and no user is present.
pub fn install_unauth_redirect<F>(auth: RwSignal<AuthState>, navigate: F)
where
    F: Fn(&str, NavigateOptions) + Clone + 'static,
{
    let navigate = navigate.clone();
    Effect::new(move || {
        let state = auth.get();
        if should_redirect_unauth(&state) {
            #[cfg(feature = "hydrate")]
            if let Some(window) = web_sys::window() {
                if let Ok(pathname) = window.location().pathname()
                    && pathname != "/login"
                {
                    let _ = window.location().set_href("/login");
                    return;
                }
            }
            navigate("/login", NavigateOptions::default());
        }
    });
}
