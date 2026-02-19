//! Login page with GitHub OAuth redirect button.
//!
//! DESIGN
//! ======
//! Kept intentionally minimal: auth complexity stays server-side and this view
//! remains a stable handoff surface to `/auth/github`.

use leptos::prelude::*;

/// Login page — clicking the button navigates to the GitHub OAuth endpoint.
#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="login-page">
            <div class="login-card">
                <h1>"Gauntlet AI"</h1>
                <h2 class="login-card__title">"G4 - Ian Zepp - Week 1"</h2>
                <a href="/auth/github" class="login-button">
                    "Sign in with GitHub"
                </a>
            </div>
        </div>
    }
}
