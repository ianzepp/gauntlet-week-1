//! Dashboard page listing boards with create and open actions.
//!
//! SYSTEM CONTEXT
//! ==============
//! This is the authenticated landing route. It requests board inventory over
//! websocket once connectivity is ready and coordinates create->navigate flow.

#[cfg(test)]
#[path = "dashboard_test.rs"]
mod dashboard_test;

use leptos::prelude::*;
use leptos::tachys::view::any_view::IntoAny;
use leptos_router::hooks::use_navigate;

use crate::app::FrameSender;
use crate::components::board_card::BoardCard;
use crate::state::auth::AuthState;
use crate::state::board::{BoardState, ConnectionStatus};
use crate::state::boards::BoardListItem;
use crate::state::boards::BoardsState;
use crate::state::ui::UiState;
use crate::util::auth::install_unauth_redirect;
use crate::util::frame::request_frame;

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

    install_unauth_redirect(auth, navigate.clone());

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

    // Create-board dialog state.
    let show_create = RwSignal::new(false);
    let new_board_name = RwSignal::new(String::new());
    let delete_board_id = RwSignal::new(None::<String>);

    // Join-board dialog state.
    let show_join = RwSignal::new(false);
    let join_code = RwSignal::new(String::new());

    let on_create = Callback::new(move |_| {
        show_create.set(true);
        new_board_name.set(String::new());
    });

    let on_cancel = Callback::new(move |_| show_create.set(false));
    let on_delete_cancel = Callback::new(move |_| delete_board_id.set(None));
    let on_board_delete_request = Callback::new(move |id: String| delete_board_id.set(Some(id)));

    let on_join = Callback::new(move |_| {
        show_join.set(true);
        join_code.set(String::new());
    });
    let on_join_cancel = Callback::new(move |_| show_join.set(false));

    let navigate_to_board = navigate.clone();
    Effect::new(move || {
        if let Some(board_id) = boards.get().created_board_id.clone() {
            boards.update(|s| s.created_board_id = None);
            navigate_to_board(&format!("/board/{board_id}"), leptos_router::NavigateOptions::default());
        }
    });

    let navigate_to_redeemed = navigate.clone();
    Effect::new(move || {
        if let Some(board_id) = boards.get().redeemed_board_id.clone() {
            boards.update(|s| s.redeemed_board_id = None);
            show_join.set(false);
            navigate_to_redeemed(&format!("/board/{board_id}"), leptos_router::NavigateOptions::default());
        }
    });

    let on_logout = Callback::new(move |_| {
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
    });

    view! {
        {move || {
            if !auth.get().loading && auth.get().user.is_some() {
                view! {
                    <div class="dashboard-page">
                        <DashboardHeader ui=ui auth=auth on_create=on_create on_join=on_join on_logout=on_logout />
                        <DashboardGrid boards=boards auth=auth on_board_delete_request=on_board_delete_request />
                        <DashboardDialogs
                            show_create=show_create
                            new_board_name=new_board_name
                            on_cancel=on_cancel
                            delete_board_id=delete_board_id
                            on_delete_cancel=on_delete_cancel
                            show_join=show_join
                            join_code=join_code
                            on_join_cancel=on_join_cancel
                            boards=boards
                            sender=sender
                        />
                    </div>
                }
                    .into_any()
            } else {
                view! { <DashboardAuthFallback auth=auth /> }.into_any()
            }
        }}
    }
}

#[component]
fn DashboardAuthFallback(auth: RwSignal<AuthState>) -> impl IntoView {
    view! {
        <div class="dashboard-page">
            <p>{move || if auth.get().loading { "Loading..." } else { "Redirecting to login..." }}</p>
        </div>
    }
}

#[component]
fn DashboardHeader(
    ui: RwSignal<UiState>,
    auth: RwSignal<AuthState>,
    on_create: Callback<()>,
    on_join: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    let self_name = move || {
        auth.get()
            .user
            .map(|user| user.name)
            .unwrap_or_else(|| "me".to_owned())
    };
    let self_method = move || {
        auth.get()
            .user
            .map(|user| user.auth_method)
            .unwrap_or_else(|| "session".to_owned())
    };

    view! {
        <header class="dashboard-page__header toolbar">
            <span class="toolbar__board-name">"Boards"</span>

            <div class="toolbar__segment" role="group" aria-label="Theme mode">
                <button
                    class="btn toolbar__segment-btn"
                    class:toolbar__segment-btn--active=move || !ui.get().dark_mode
                    on:click=move |_| {
                        if ui.get().dark_mode {
                            let next = crate::util::dark_mode::toggle(true);
                            ui.update(|u| u.dark_mode = next);
                        }
                    }
                    title="Light mode"
                >
                    "Light"
                </button>
                <button
                    class="btn toolbar__segment-btn"
                    class:toolbar__segment-btn--active=move || ui.get().dark_mode
                    on:click=move |_| {
                        if !ui.get().dark_mode {
                            let next = crate::util::dark_mode::toggle(false);
                            ui.update(|u| u.dark_mode = next);
                        }
                    }
                    title="Dark mode"
                >
                    "Dark"
                </button>
            </div>

            <button class="btn toolbar__join-board" on:click=move |_| on_create.run(())>
                "+ New Board"
            </button>
            <button class="btn toolbar__join-board" on:click=move |_| on_join.run(())>
                "Join Board"
            </button>

            <span class="toolbar__spacer"></span>

            <span class="toolbar__self">
                {self_name}
                " ("
                <span class="toolbar__self-method">{self_method}</span>
                ")"
            </span>

            <button class="btn toolbar__logout" on:click=move |_| on_logout.run(()) title="Logout">
                "Logout"
            </button>
        </header>
    }
}

#[component]
fn DashboardGrid(
    boards: RwSignal<BoardsState>,
    auth: RwSignal<AuthState>,
    on_board_delete_request: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="dashboard-page__grid">
            {move || {
                boards
                    .get()
                    .error
                    .map(|error| view! { <p class="dashboard-page__error">{error}</p> }.into_any())
                    .unwrap_or_else(|| view! { <></> }.into_any())
            }}
            {move || {
                if boards.get().loading {
                    view! { <p>"Loading boards..."</p> }.into_any()
                } else {
                    view! { <BoardSections boards=boards auth=auth on_board_delete_request=on_board_delete_request /> }
                        .into_any()
                }
            }}
        </div>
    }
}

#[component]
fn BoardSections(
    boards: RwSignal<BoardsState>,
    auth: RwSignal<AuthState>,
    on_board_delete_request: Callback<String>,
) -> impl IntoView {
    view! {
        {move || {
            let state = boards.get();
            let my_user_id = auth.get().user.map(|u| u.id).unwrap_or_default();
            let (my_boards, shared_boards): (Vec<_>, Vec<_>) = state
                .items
                .into_iter()
                .partition(|b| b.owner_id.as_deref() == Some(my_user_id.as_str()));
            view! {
                <BoardSection title="My Boards" items=my_boards on_delete=on_board_delete_request.clone() />
                <BoardSection title="Shared Boards" items=shared_boards on_delete=on_board_delete_request.clone() />
            }
        }}
    }
}

#[component]
fn BoardSection(title: &'static str, items: Vec<BoardListItem>, on_delete: Callback<String>) -> impl IntoView {
    let count = items.len();
    view! {
        <section class="dashboard-page__section">
            <header class="dashboard-page__section-header">
                <h2 class="dashboard-page__section-title">{title}</h2>
                <span class="dashboard-page__section-count">{count}</span>
            </header>
            <div class="dashboard-page__cards">
                {items
                    .into_iter()
                    .map(|b| {
                        view! {
                            <BoardCard id=b.id name=b.name snapshot=b.snapshot on_delete=on_delete />
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
        </section>
    }
}

#[component]
fn DashboardDialogs(
    show_create: RwSignal<bool>,
    new_board_name: RwSignal<String>,
    on_cancel: Callback<()>,
    delete_board_id: RwSignal<Option<String>>,
    on_delete_cancel: Callback<()>,
    show_join: RwSignal<bool>,
    join_code: RwSignal<String>,
    on_join_cancel: Callback<()>,
    boards: RwSignal<BoardsState>,
    sender: RwSignal<FrameSender>,
) -> impl IntoView {
    view! {
        {move || {
            if show_create.get() {
                view! { <CreateBoardDialog name=new_board_name on_cancel=on_cancel boards=boards sender=sender /> }
                    .into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
        {move || {
            if delete_board_id.get().is_some() {
                view! { <DeleteBoardDialog board_id=delete_board_id on_cancel=on_delete_cancel boards=boards sender=sender /> }
                    .into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
        {move || {
            if show_join.get() {
                view! { <JoinBoardDialog code=join_code on_cancel=on_join_cancel sender=sender /> }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
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
                    <input
                        class="dialog__input"
                        type="text"
                        autofocus=true
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
    let frame = build_board_list_frame(boards.get_untracked().list_rev.clone());
    let _ = sender.get_untracked().send(&frame);
}

fn send_board_create(sender: RwSignal<FrameSender>, name: &str) {
    let frame = build_board_create_frame(name);
    let _ = sender.get_untracked().send(&frame);
}

fn send_board_delete(sender: RwSignal<FrameSender>, board_id: &str) {
    let frame = build_board_delete_frame(board_id);
    let _ = sender.get_untracked().send(&frame);
}

fn send_access_redeem(sender: RwSignal<FrameSender>, code: &str) {
    let frame = build_access_redeem_frame(code);
    let _ = sender.get_untracked().send(&frame);
}

fn build_board_list_frame(since_rev: Option<String>) -> crate::net::types::Frame {
    request_frame(
        "board:list",
        None,
        serde_json::json!({
            "since_rev": since_rev
        }),
    )
}

fn build_board_create_frame(name: &str) -> crate::net::types::Frame {
    request_frame("board:create", None, serde_json::json!({ "name": name }))
}

fn build_board_delete_frame(board_id: &str) -> crate::net::types::Frame {
    request_frame("board:delete", None, serde_json::json!({ "board_id": board_id }))
}

fn build_access_redeem_frame(code: &str) -> crate::net::types::Frame {
    request_frame("board:access:redeem", None, serde_json::json!({ "code": code }))
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
