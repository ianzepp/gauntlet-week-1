//! Login page with GitHub OAuth redirect button.

use leptos::prelude::*;

/// Login page — clicking the button navigates to the GitHub OAuth endpoint.
#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="login-page">
            <div class="login-card">
                <h1 class="login-card__title">"Gauntlet"</h1>
                <p class="login-card__subtitle">"Collaborative whiteboard"</p>
                <a href="/auth/github" class="login-button">
                    "Sign in with GitHub"
                </a>
            </div>
        </div>
    }
}
