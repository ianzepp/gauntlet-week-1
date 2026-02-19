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
use crate::state::ui::UiState;

/// Dashboard page — shows a board list and a create-board button.
/// Redirects to `/login` if the user is not authenticated.
#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let boards = expect_context::<RwSignal<BoardsState>>();
    let ui = expect_context::<RwSignal<UiState>>();
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
        send_board_list(sender, boards);
        requested_list.set(true);
    });

    #[cfg(feature = "hydrate")]
    {
        let poll_alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let poll_alive_task = poll_alive.clone();
        let board_poll = board;
        let boards_poll = boards;
        let sender_poll = sender;
        leptos::task::spawn_local(async move {
            loop {
                gloo_timers::future::sleep(std::time::Duration::from_secs(10)).await;
                if !poll_alive_task.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                if !matches!(board_poll.get_untracked().connection_status, ConnectionStatus::Connected) {
                    continue;
                }
                send_board_list(sender_poll, boards_poll);
            }
        });
        on_cleanup(move || poll_alive.store(false, std::sync::atomic::Ordering::Relaxed));
    }

    // Create-board dialog state.
    let show_create = RwSignal::new(false);
    let new_board_name = RwSignal::new(String::new());
    let delete_board_id = RwSignal::new(None::<String>);

    // Join-board dialog state.
    let show_join = RwSignal::new(false);
    let join_code = RwSignal::new(String::new());

    let on_create = move |_| {
        show_create.set(true);
        new_board_name.set(String::new());
    };

    let on_cancel = Callback::new(move |_| show_create.set(false));
    let on_delete_cancel = Callback::new(move |_| delete_board_id.set(None));
    let on_board_delete_request = Callback::new(move |id: String| delete_board_id.set(Some(id)));

    let on_join = move |_| {
        show_join.set(true);
        join_code.set(String::new());
    };
    let on_join_cancel = Callback::new(move |_| show_join.set(false));

    let navigate_to_board = navigate.clone();
    Effect::new(move || {
        if let Some(board_id) = boards.get().created_board_id.clone() {
            boards.update(|s| s.created_board_id = None);
            navigate_to_board(&format!("/board/{board_id}"), NavigateOptions::default());
        }
    });

    let navigate_to_redeemed = navigate.clone();
    Effect::new(move || {
        if let Some(board_id) = boards.get().redeemed_board_id.clone() {
            boards.update(|s| s.redeemed_board_id = None);
            show_join.set(false);
            navigate_to_redeemed(&format!("/board/{board_id}"), NavigateOptions::default());
        }
    });

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
        <Show
            when=move || !auth.get().loading && auth.get().user.is_some()
            fallback=move || {
                view! {
                    <div class="dashboard-page">
                        <p>{move || if auth.get().loading { "Loading..." } else { "Redirecting to login..." }}</p>
                    </div>
                }
            }
        >
            <div class="dashboard-page">
                <header class="dashboard-page__header toolbar">
                    <span class="toolbar__board-name">"Boards"</span>
                    <span class="toolbar__divider" aria-hidden="true"></span>
                    <button class="btn toolbar__new-board" on:click=on_create>
                        "+ New Board"
                    </button>
                    <button class="btn toolbar__join-board" on:click=on_join>
                        "Join Board"
                    </button>

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
                            {move || {
                                boards
                                    .get()
                                    .items
                                    .into_iter()
                                    .map(|b| {
                                        view! {
                                            <BoardCard
                                                id=b.id
                                                name=b.name
                                                snapshot=b.snapshot
                                                on_delete=on_board_delete_request
                                            />
                                        }
                                    })
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
                <Show when=move || delete_board_id.get().is_some()>
                    <DeleteBoardDialog
                        board_id=delete_board_id
                        on_cancel=on_delete_cancel
                        boards=boards
                        sender=sender
                    />
                </Show>
                <Show when=move || show_join.get()>
                    <JoinBoardDialog
                        code=join_code
                        on_cancel=on_join_cancel
                        sender=sender
                    />
                </Show>
            </div>
        </Show>
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

fn send_board_list(sender: RwSignal<FrameSender>, boards: RwSignal<BoardsState>) {
    let since_rev = boards.get_untracked().list_rev;
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "since_rev": since_rev
        }),
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

fn send_board_delete(sender: RwSignal<FrameSender>, board_id: &str) {
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:delete".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({ "board_id": board_id }),
    };
    let _ = sender.get_untracked().send(&frame);
}

fn send_access_redeem(sender: RwSignal<FrameSender>, code: &str) {
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:access:redeem".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({ "code": code }),
    };
    let _ = sender.get_untracked().send(&frame);
}

#[component]
fn DeleteBoardDialog(
    board_id: RwSignal<Option<String>>,
    on_cancel: Callback<()>,
    boards: RwSignal<BoardsState>,
    sender: RwSignal<FrameSender>,
) -> impl IntoView {
    let submit = Callback::new(move |_| {
        let Some(id) = board_id.get_untracked() else {
            return;
        };
        boards.update(|s| s.loading = true);
        send_board_delete(sender, &id);
        on_cancel.run(());
    });

    view! {
        <div class="dialog-backdrop" on:click=move |_| on_cancel.run(())>
            <div class="dialog" on:click=move |ev| ev.stop_propagation()>
                <h2>"Delete Board"</h2>
                <p class="dialog__danger">
                    "This will permanently delete this board and its objects."
                </p>
                <div class="dialog__actions">
                    <button class="btn" on:click=move |_| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button class="btn btn--danger" on:click=move |_| submit.run(())>
                        "Delete"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Modal dialog for entering a board access code to join.
#[component]
fn JoinBoardDialog(code: RwSignal<String>, on_cancel: Callback<()>, sender: RwSignal<FrameSender>) -> impl IntoView {
    let submit = Callback::new(move |_| {
        let value = code.get();
        if value.trim().is_empty() {
            return;
        }
        send_access_redeem(sender, value.trim());
    });

    view! {
        <div class="dialog-backdrop" on:click=move |_| on_cancel.run(())>
            <div class="dialog" on:click=move |ev| ev.stop_propagation()>
                <h2>"Join Board"</h2>
                <label class="dialog__label">
                    "Access Code"
                    <input
                        class="dialog__input"
                        type="text"
                        maxlength="6"
                        placeholder="e.g. ABC123"
                        prop:value=move || code.get()
                        on:input=move |ev| {
                            code.set(event_target_value(&ev).to_ascii_uppercase());
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
                        "Join"
                    </button>
                </div>
            </div>
        </div>
    }
}
