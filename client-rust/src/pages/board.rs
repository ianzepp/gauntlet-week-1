//! Board page — the main workspace layout.

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
use crate::net::types::{Frame, FrameStatus};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;

/// Board page — composes toolbar, panels, canvas placeholder, and status bar
/// in a CSS grid layout. Reads the board ID from the route parameter and
/// updates `BoardState` on mount.
#[component]
pub fn BoardPage() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let params = use_params_map();
    let last_join_key = RwSignal::new(None::<(String, String)>);

    // Extract board ID from route.
    let board_id = move || params.read().get("id");

    // Update board state when the route param changes.
    Effect::new(move || {
        let id = board_id();
        board.update(|b| {
            b.board_id.clone_from(&id);
            b.board_name = None;
            b.self_client_id = None;
            b.follow_client_id = None;
            b.jump_to_client_id = None;
            b.objects.clear();
            b.savepoints.clear();
            b.drag_objects.clear();
            b.drag_updated_at.clear();
            b.cursor_updated_at.clear();
            b.selection.clear();
            b.presence.clear();
        });
        last_join_key.set(None);
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

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id),
            from: None,
            syscall: "board:join".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({}),
        };
        sender.get().send(&frame);
        last_join_key.set(Some(key));
    });

    on_cleanup(move || {
        let board_id = board.get().board_id;
        if let Some(board_id) = board_id {
            let frame = Frame {
                id: uuid::Uuid::new_v4().to_string(),
                parent_id: None,
                ts: 0,
                board_id: Some(board_id),
                from: None,
                syscall: "board:part".to_owned(),
                status: FrameStatus::Request,
                data: serde_json::json!({}),
            };
            sender.get().send(&frame);
        }

        board.update(|b| {
            b.board_id = None;
            b.board_name = None;
            b.self_client_id = None;
            b.follow_client_id = None;
            b.jump_to_client_id = None;
            b.objects.clear();
            b.savepoints.clear();
            b.drag_objects.clear();
            b.drag_updated_at.clear();
            b.cursor_updated_at.clear();
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
        <div class="board-page">
            <div class="board-page__toolbar">
                <Toolbar/>
            </div>
            <div class="board-page__left-panel">
                <LeftPanel/>
            </div>
            <div class="board-page__canvas">
                <CanvasHost/>
                <BoardStamp/>
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
