//! Bottom status bar showing connection status, zoom level, and object count.

use leptos::prelude::*;

use crate::state::board::{BoardState, ConnectionStatus};

/// Status bar at the bottom of the board page.
///
/// Shows connection status indicator, board name, object count, and
/// placeholders for zoom level and cursor position (canvas-dependent).
#[component]
pub fn StatusBar() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();

    let status_class = move || {
        let status = board.get().connection_status;
        match status {
            ConnectionStatus::Connected => "status-bar__dot status-bar__dot--connected",
            ConnectionStatus::Connecting => "status-bar__dot status-bar__dot--connecting",
            ConnectionStatus::Disconnected => "status-bar__dot status-bar__dot--disconnected",
        }
    };

    let status_label = move || {
        let status = board.get().connection_status;
        match status {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Disconnected => "Disconnected",
        }
    };

    let object_count = move || board.get().objects.len();
    let board_name = move || board.get().board_name.unwrap_or_default();

    view! {
        <div class="status-bar">
            <span class="status-bar__connection">
                <span class=status_class></span>
                {status_label}
            </span>
            <span class="status-bar__divider">"|"</span>
            <span class="status-bar__board-name">{board_name}</span>
            <span class="status-bar__divider">"|"</span>
            <span class="status-bar__objects">{move || format!("{} objects", object_count())}</span>
            <span class="status-bar__spacer"></span>
            <span class="status-bar__zoom">"100%"</span>
        </div>
    }
}
