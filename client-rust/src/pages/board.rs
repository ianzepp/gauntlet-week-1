//! Board page — the main workspace layout.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// Board page — composes toolbar, panels, canvas placeholder, and status bar.
/// Reads the board ID from the route parameter.
#[component]
pub fn BoardPage() -> impl IntoView {
    let params = use_params_map();
    let board_id = move || params.read().get("id");

    view! {
        <div class="board-page">
            <div class="board-page__toolbar">"Toolbar"</div>
            <div class="board-page__left-panel">"Left Panel"</div>
            <div class="board-page__canvas">
                {move || {
                    let id = board_id().unwrap_or_default();
                    format!("Canvas placeholder (board: {id})")
                }}
            </div>
            <div class="board-page__right-panel">"Right Panel"</div>
            <div class="board-page__status-bar">"Status Bar"</div>
        </div>
    }
}
