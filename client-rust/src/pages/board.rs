//! Board page — the main workspace layout.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_params_map;

use crate::components::left_panel::LeftPanel;
use crate::components::right_panel::RightPanel;
use crate::components::status_bar::StatusBar;
use crate::components::toolbar::Toolbar;
use crate::state::auth::AuthState;
use crate::state::board::BoardState;

/// Board page — composes toolbar, panels, canvas placeholder, and status bar
/// in a CSS grid layout. Reads the board ID from the route parameter and
/// updates `BoardState` on mount.
#[component]
pub fn BoardPage() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let params = use_params_map();

    // Extract board ID from route.
    let board_id = move || params.read().get("id");

    // Update board state when the route param changes.
    Effect::new(move || {
        let id = board_id();
        board.update(|b| {
            b.board_id.clone_from(&id);
            b.board_name = None;
        });
        // TODO: send board:join via frame client
    });

    // Redirect to login if not authenticated.
    let navigate = leptos_router::hooks::use_navigate();
    Effect::new(move || {
        let state = auth.get();
        if !state.loading && state.user.is_none() {
            navigate("/login", NavigateOptions::default());
        }
    });

    view! {
        <div class="board-page">
            <div class="board-page__toolbar">
                <Toolbar/>
            </div>
            <div class="board-page__left-panel">
                <LeftPanel/>
            </div>
            <div class="board-page__canvas">
                {move || {
                    let id = board_id().unwrap_or_default();
                    format!("Canvas placeholder (board: {id})")
                }}
            </div>
            <div class="board-page__right-panel">
                <RightPanel/>
            </div>
            <div class="board-page__status-bar">
                <StatusBar/>
            </div>
        </div>
    }
}
