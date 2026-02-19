//! Top bar displaying board name, presence avatars, back button, and logout.
//!
//! SYSTEM CONTEXT
//! ==============
//! This component surfaces session/board metadata and primary navigation
//! controls that remain visible during board workflows.

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::ui::UiState;

/// Top toolbar for the board page.
#[component]
pub fn Toolbar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let location = use_location();

    let board_name = move || {
        board
            .get()
            .board_name
            .unwrap_or_else(|| "Untitled".to_owned())
    };

    let self_identity = move || {
        auth.get()
            .user
            .map(|user| (user.name, user.auth_method))
            .unwrap_or_else(|| ("me".to_owned(), "session".to_owned()))
    };

    let on_logout = move |_| {
        #[cfg(feature = "hydrate")]
        {
            leptos::task::spawn_local(async move {
                crate::net::api::logout().await;
                auth.update(|a| a.user = None);
                if let Some(w) = web_sys::window() {
                    let _ = w.location().set_href("/login");
                }
            });
        }
    };

    view! {
        <div class="toolbar">
            <Show when=move || location.pathname.get().starts_with("/board/")>
                <a href="/" class="toolbar__back" title="Back to dashboard">
                    "←"
                </a>
            </Show>

            <span class="toolbar__board-name">{board_name}</span>
            <span class="toolbar__divider"></span>

            <span class="toolbar__spacer"></span>

            <button
                class="btn toolbar__dark-toggle"
                on:click=move |_| {
                    let current = ui.get().dark_mode;
                    let next = crate::util::dark_mode::toggle(current);
                    ui.update(|u| u.dark_mode = next);
                }
                title="Toggle dark mode"
            >
                {move || if ui.get().dark_mode { "☀" } else { "☾" }}
            </button>

            <span class="toolbar__self">
                {move || self_identity().0}
                " ("
                <span class="toolbar__self-method">{move || self_identity().1}</span>
                ")"
            </span>

            <button class="btn toolbar__logout" on:click=on_logout title="Logout">
                "Logout"
            </button>
        </div>
    }
}
