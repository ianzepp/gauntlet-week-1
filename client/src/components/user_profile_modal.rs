//! Modal displaying the current user's profile and session token.

use leptos::prelude::*;

use crate::state::auth::AuthState;

/// User profile modal with name, auth method, user ID, and copyable session token.
#[component]
pub fn UserProfileModal(auth: RwSignal<AuthState>, on_close: Callback<()>) -> impl IntoView {
    let token = RwSignal::new(None::<String>);
    let copied = RwSignal::new(false);

    // Fetch session token on mount.
    #[cfg(feature = "hydrate")]
    {
        leptos::task::spawn_local(async move {
            if let Some(t) = crate::net::api::fetch_session_token().await {
                token.set(Some(t));
            }
        });
    }

    let on_backdrop = move |_| on_close.run(());
    let on_close_click = move |_| on_close.run(());
    let on_keydown = Callback::new(move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            on_close.run(());
        }
    });

    let on_copy = move |_| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(t) = token.get_untracked() {
                if let Some(window) = web_sys::window() {
                    if let Some(clipboard) = window.navigator().clipboard() {
                        let _ = clipboard.write_text(&t);
                        copied.set(true);
                    }
                }
            }
        }
    };

    let user_name = move || {
        auth.get()
            .user
            .as_ref()
            .map_or_else(|| "—".to_owned(), |u| u.name.clone())
    };
    let user_method = move || {
        auth.get()
            .user
            .as_ref()
            .map_or_else(|| "—".to_owned(), |u| u.auth_method.clone())
    };
    let user_id = move || {
        auth.get()
            .user
            .as_ref()
            .map_or_else(|| "—".to_owned(), |u| u.id.clone())
    };

    view! {
        <div class="dialog-backdrop" on:click=on_backdrop>
            <div
                class="dialog dialog--profile"
                on:click=move |ev| ev.stop_propagation()
                on:keydown=move |ev| on_keydown.run(ev)
                tabindex="0"
            >
                <h2>"User Profile"</h2>

                <div class="dialog__profile-row">
                    <span class="dialog__profile-label">"Name"</span>
                    <span class="dialog__profile-value">{user_name}</span>
                </div>
                <div class="dialog__profile-row">
                    <span class="dialog__profile-label">"Auth"</span>
                    <span class="dialog__profile-value">{user_method}</span>
                </div>
                <div class="dialog__profile-row">
                    <span class="dialog__profile-label">"User ID"</span>
                    <span class="dialog__profile-value dialog__profile-value--mono">{user_id}</span>
                </div>

                <div class="dialog__profile-row">
                    <span class="dialog__profile-label">"Session Token"</span>
                </div>
                <div class="dialog__profile-token-box">
                    <code class="dialog__profile-token-text">
                        {move || token.get().unwrap_or_else(|| "Loading…".to_owned())}
                    </code>
                    <button class="btn dialog__profile-token-copy" on:click=on_copy title="Copy token">
                        {move || if copied.get() { "Copied" } else { "Copy" }}
                    </button>
                </div>

                <div class="dialog__actions">
                    <button class="btn btn--primary" on:click=on_close_click>"Close"</button>
                </div>
            </div>
        </div>
    }
}
