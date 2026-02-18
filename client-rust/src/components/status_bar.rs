//! Bottom status bar showing connection status, zoom level, and object count.

use leptos::prelude::*;

use crate::state::auth::AuthState;
use crate::state::board::{BoardState, ConnectionStatus};

/// Status bar at the bottom of the board page.
#[component]
pub fn StatusBar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();

    let status_class = move || {
        let status = board.get().connection_status;
        match status {
            ConnectionStatus::Connected => "status-bar__dot status-bar__dot--connected",
            ConnectionStatus::Connecting => "status-bar__dot status-bar__dot--connecting",
            ConnectionStatus::Disconnected => "status-bar__dot status-bar__dot--disconnected",
        }
    };

    let object_count = move || board.get().objects.len();
    let board_name = move || board.get().board_name.unwrap_or_default();

    let user = move || auth.get().user;

    view! {
        <div class="status-bar">
            <div class="status-bar__section">
                <span class="status-bar__item">
                    <span class=status_class></span>
                </span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__board-name">{board_name}</span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__item">{move || format!("{} objs", object_count())}</span>
            </div>

            <div class="status-bar__section">
                <span class="status-bar__item">"(0, 0)"</span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__item">"(0, 0)"</span>

                <span class="status-bar__divider"></span>
                <Show when=move || user().is_some()>
                    <span class="status-bar__user-chip">
                        <span class="status-bar__user-dot" style:background=move || user().map_or_else(String::new, |u| u.color)></span>
                        {move || user().map_or_else(String::new, |u| u.name)}
                    </span>
                    <span class="status-bar__divider"></span>
                </Show>

                <span class="status-bar__item">"100%"</span>
            </div>
        </div>
    }
}
