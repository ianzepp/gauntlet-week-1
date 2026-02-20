//! Board page — the interactive board workspace shell.
//!
//! ARCHITECTURE
//! ============
//! This component is the route-level coordinator between URL board identity,
//! websocket board membership (`board:join`/`board:part`), and local
//! `BoardState` cache lifecycle.
//!
//! SYSTEM CONTEXT
//! ==============
//! The frame client owns websocket connection/session identity. `BoardPage`
//! translates route transitions into board membership transitions without
//! requiring websocket reconnects.
//!
//! TRADE-OFFS
//! ==========
//! We preserve `self_client_id` across route changes so membership transitions
//! stay valid on the same websocket session. This favors continuity/correctness
//! over aggressive full-state resets.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_params_map;

use crate::app::FrameSender;
use crate::components::board_stamp::BoardStamp;
use crate::components::canvas_host::CanvasHost;
use crate::components::left_panel::LeftPanel;
use crate::components::right_panel::RightPanel;
use crate::components::status_bar::StatusBar;
use crate::components::toolbar::Toolbar;
use crate::components::trace_view::TraceView;
use crate::net::types::{Frame, FrameStatus};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::ui::{UiState, ViewMode};

fn build_board_membership_frame(syscall: &str, board_id: String) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: syscall.to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({}),
    }
}

fn reset_board_for_route_change(board: &mut BoardState, next_board_id: Option<String>) {
    board.board_id = next_board_id;
    board.board_name = None;
    // WHY: websocket session identity is stable across board route changes.
    // Clearing this breaks subsequent board:join transitions.
    board.follow_client_id = None;
    board.jump_to_client_id = None;
    board.objects.clear();
    board.savepoints.clear();
    board.drag_objects.clear();
    board.drag_updated_at.clear();
    board.cursor_updated_at.clear();
    board.join_streaming = false;
    board.selection.clear();
    board.presence.clear();
}

/// Board page — composes toolbar, panels, canvas placeholder, and status bar
/// in a CSS grid layout. Reads the board ID from the route parameter and
/// updates `BoardState` on mount.
#[component]
pub fn BoardPage() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let params = use_params_map();
    let last_join_key = RwSignal::new(None::<(String, String)>);
    let last_route_board_id = RwSignal::new(None::<String>);

    // Extract board ID from route.
    let board_id = move || params.read().get("id");

    // Update board state when the route param changes.
    Effect::new(move || {
        let next_id = board_id();
        let prev_id = last_route_board_id.get_untracked();
        if prev_id == next_id {
            return;
        }

        // PHASE: PART PREVIOUS BOARD MEMBERSHIP
        // WHY: route changes do not unmount this component, so explicit part is
        // required to prevent stale presence on the previous board.
        if let Some(previous_board_id) = prev_id.clone() {
            sender
                .get()
                .send(&build_board_membership_frame("board:part", previous_board_id));
        }

        // PHASE: RESET ROUTE-SCOPED BOARD CACHE
        // WHY: board data is board-id scoped, but websocket client identity is
        // connection-scoped and intentionally preserved.
        board.update(|b| reset_board_for_route_change(b, next_id.clone()));
        last_join_key.set(None);
        last_route_board_id.set(next_id);
    });

    // Send board:join once per (board_id, websocket client_id), including reconnects.
    Effect::new(move || {
        let state = board.get();
        if state.connection_status != crate::state::board::ConnectionStatus::Connected {
            return;
        }
        let Some(board_id) = state.board_id.clone() else {
            return;
        };
        let Some(client_id) = state.self_client_id.clone() else {
            return;
        };
        let key = (board_id.clone(), client_id.clone());
        if last_join_key.get().as_ref() == Some(&key) {
            return;
        }

        sender
            .get()
            .send(&build_board_membership_frame("board:join", board_id));
        last_join_key.set(Some(key));
    });

    on_cleanup(move || {
        let board_id = board.get().board_id;
        if let Some(board_id) = board_id {
            sender
                .get()
                .send(&build_board_membership_frame("board:part", board_id));
        }

        board.update(|b| {
            b.board_id = None;
            b.board_name = None;
            b.follow_client_id = None;
            b.jump_to_client_id = None;
            b.objects.clear();
            b.savepoints.clear();
            b.drag_objects.clear();
            b.drag_updated_at.clear();
            b.cursor_updated_at.clear();
            b.join_streaming = false;
            b.selection.clear();
            b.presence.clear();
        });
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
        <div
            class="board-page"
            class:board-page--left-expanded=move || ui.get().left_panel_expanded
            class:board-page--right-expanded=move || ui.get().right_panel_expanded
            class:board-page--trace=move || ui.get().view_mode == ViewMode::Trace
        >
            <div class="board-page__toolbar">
                <Toolbar/>
            </div>
            <Show when=move || ui.get().view_mode == ViewMode::Canvas>
                <div class="board-page__left-panel">
                    <LeftPanel/>
                </div>
            </Show>
            <div class="board-page__canvas">
                <Show
                    when=move || ui.get().view_mode == ViewMode::Canvas
                    fallback=|| view! { <TraceView/> }
                >
                    <CanvasHost/>
                    <BoardStamp/>
                </Show>
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

#[cfg(test)]
#[path = "board_test.rs"]
mod board_test;
