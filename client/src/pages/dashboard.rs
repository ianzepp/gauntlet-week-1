//! Dashboard page listing boards with create and open actions.
//!
//! SYSTEM CONTEXT
//! ==============
//! This is the authenticated landing route. It requests board inventory over
//! websocket once connectivity is ready and coordinates create->navigate flow.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::app::FrameSender;
use crate::components::board_card::BoardCard;
use crate::net::types::{Frame, FrameStatus};
use crate::state::auth::AuthState;
use crate::state::board::{BoardState, ConnectionStatus};
use crate::state::boards::BoardsState;

/// Dashboard page — shows a board list and a create-board button.
/// Redirects to `/login` if the user is not authenticated.
#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let boards = expect_context::<RwSignal<BoardsState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let navigate = use_navigate();

    // Redirect to login if not authenticated.
    let navigate_login = navigate.clone();
    Effect::new(move || {
        let state = auth.get();
        if !state.loading && state.user.is_none() {
            navigate_login("/login", NavigateOptions::default());
        }
    });

    let requested_list = RwSignal::new(false);
    Effect::new(move || {
        if requested_list.get() {
            return;
        }
        if !matches!(board.get().connection_status, ConnectionStatus::Connected) {
            return;
        }
        boards.update(|s| s.loading = true);
        send_board_list(sender);
        requested_list.set(true);
    });

    // Create-board dialog state.
    let show_create = RwSignal::new(false);
    let new_board_name = RwSignal::new(String::new());

    let on_create = move |_| {
        show_create.set(true);
        new_board_name.set(String::new());
    };

    let on_cancel = Callback::new(move |_| show_create.set(false));

    let navigate_to_board = navigate.clone();
    Effect::new(move || {
        if let Some(board_id) = boards.get().created_board_id.clone() {
            boards.update(|s| s.created_board_id = None);
            navigate_to_board(&format!("/board/{board_id}"), NavigateOptions::default());
        }
    });

    view! {
        <div class="dashboard-page">
            <header class="dashboard-page__header">
                <h1>"Boards"</h1>
                <button class="btn btn--primary" on:click=on_create>
                    "+ New Board"
                </button>
            </header>

            <div class="dashboard-page__grid">
                <Show when=move || boards.get().error.is_some()>
                    <p class="dashboard-page__error">
                        {move || boards.get().error.unwrap_or_default()}
                    </p>
                </Show>
                <Show
                    when=move || !boards.get().loading
                    fallback=move || view! { <p>"Loading boards..."</p> }
                >
                    <div class="dashboard-page__cards">
                        <button class="dashboard-page__new-card" on:click=on_create title="Create board">
                            <svg class="dashboard-page__new-icon" viewBox="0 0 20 20" aria-hidden="true">
                                <line x1="10" y1="4" x2="10" y2="16"></line>
                                <line x1="4" y1="10" x2="16" y2="10"></line>
                            </svg>
                        </button>
                        {move || {
                            boards
                                .get()
                                .items
                                .into_iter()
                                .map(|b| view! { <BoardCard id=b.id name=b.name snapshot=b.snapshot/> })
                                .collect::<Vec<_>>()
                        }}
                    </div>
                </Show>
            </div>

            <Show when=move || show_create.get()>
                <CreateBoardDialog
                    name=new_board_name
                    on_cancel=on_cancel
                    boards=boards
                    sender=sender
                />
            </Show>
        </div>
    }
}

/// Modal dialog for creating a new board.
#[component]
fn CreateBoardDialog(
    name: RwSignal<String>,
    on_cancel: Callback<()>,
    boards: RwSignal<BoardsState>,
    sender: RwSignal<FrameSender>,
) -> impl IntoView {
    let submit = Callback::new(move |_| {
        let board_name = name.get();
        if board_name.trim().is_empty() {
            return;
        }
        let board_name = board_name.trim().to_owned();
        boards.update(|s| s.create_pending = true);
        send_board_create(sender, &board_name);
        on_cancel.run(());
    });

    view! {
        <div class="dialog-backdrop" on:click=move |_| on_cancel.run(())>
            <div class="dialog" on:click=move |ev| ev.stop_propagation()>
                <h2>"Create Board"</h2>
                <label class="dialog__label">
                    "Board Name"
                    <input
                        class="dialog__input"
                        type="text"
                        prop:value=move || name.get()
                        on:input=move |ev| {
                            name.set(event_target_value(&ev));
                        }
                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                submit.run(());
                            }
                        }
                    />
                </label>
                <div class="dialog__actions">
                    <button class="btn" on:click=move |_| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button class="btn btn--primary" on:click=move |_| submit.run(())>
                        "Create"
                    </button>
                </div>
            </div>
        </div>
    }
}

fn send_board_list(sender: RwSignal<FrameSender>) {
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({}),
    };
    let _ = sender.get_untracked().send(&frame);
}

fn send_board_create(sender: RwSignal<FrameSender>, name: &str) {
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:create".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({ "name": name }),
    };
    let _ = sender.get_untracked().send(&frame);
}
