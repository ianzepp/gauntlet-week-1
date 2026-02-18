//! Top bar displaying board name, presence avatars, back button, and logout.

use leptos::prelude::*;

use crate::state::auth::AuthState;
use crate::state::board::BoardState;

/// Top toolbar for the board page.
///
/// Shows the board name, presence dots for connected users, a back button
/// to the dashboard, and a logout button.
#[component]
pub fn Toolbar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();

    let board_name = move || {
        board
            .get()
            .board_name
            .unwrap_or_else(|| "Untitled".to_owned())
    };
    let user_name = move || auth.get().user.map_or_else(String::new, |u| u.name);

    let on_logout = move |_| {
        #[cfg(feature = "hydrate")]
        {
            leptos::task::spawn_local(async move {
                crate::net::api::logout().await;
                auth.update(|a| a.user = None);
                // Navigate to login via window.location for a clean state.
                if let Some(w) = web_sys::window() {
                    let _ = w.location().set_href("/login");
                }
            });
        }
    };

    view! {
        <div class="toolbar">
            <a href="/" class="toolbar__back" title="Back to dashboard">
                "\u{2190}"
            </a>
            <span class="toolbar__board-name">{board_name}</span>
            <div class="toolbar__presence">
                {move || {
                    board
                        .get()
                        .presence
                        .values()
                        .map(|p| {
                            let border_color = p.color.clone();
                            let dot_color = p.color.clone();
                            let chip_name = p.name.clone();
                            let display_name = p.name.clone();
                            view! {
                                <span
                                    class="toolbar__presence-chip"
                                    title=chip_name
                                    style:border-color=border_color
                                >
                                    <span class="toolbar__presence-dot" style:background=dot_color></span>
                                    {display_name}
                                </span>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>
            <span class="toolbar__spacer"></span>
            <span class="toolbar__user">{user_name}</span>
            <button class="btn toolbar__logout" on:click=on_logout>
                "Logout"
            </button>
        </div>
    }
}
