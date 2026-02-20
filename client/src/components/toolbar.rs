//! Top bar displaying board name, presence avatars, back button, and logout.
//!
//! SYSTEM CONTEXT
//! ==============
//! This component surfaces session/board metadata and primary navigation
//! controls that remain visible during board workflows.

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::ui::{UiState, ViewMode};

/// Top toolbar for the board page.
#[component]
pub fn Toolbar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let location = use_location();
    let navigate = use_navigate();

    let show_share = RwSignal::new(false);

    let board_name = move || {
        board
            .get()
            .board_name
            .unwrap_or_else(|| "Untitled".to_owned())
    };
    let export_href = move || {
        board
            .get()
            .board_id
            .map(|id| format!("/api/boards/{id}/export.jsonl"))
            .unwrap_or_else(|| "#".to_owned())
    };

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

    let on_share = move |_| {
        board.update(|b| b.generated_access_code = None);
        show_share.set(true);
    };

    let on_share_cancel = Callback::new(move |_| show_share.set(false));
    let on_back = Callback::new(move |_| {
        navigate("/", leptos_router::NavigateOptions::default());
    });

    view! {
        <div class="toolbar">
            <Show when=move || location.pathname.get().starts_with("/board/")>
                <button class="toolbar__back" title="Back to dashboard" on:click=move |_| on_back.run(())>
                    "←"
                </button>
            </Show>

            <span class="toolbar__board-name">{board_name}</span>
            <Show when=move || location.pathname.get().starts_with("/board/")>
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
            </Show>

            <Show when=move || location.pathname.get().starts_with("/board/")>
                <div class="toolbar__segment" role="group" aria-label="Board view mode">
                    <button
                        class="btn toolbar__segment-btn"
                        class:toolbar__segment-btn--active=move || ui.get().view_mode == ViewMode::Canvas
                        on:click=move |_| ui.update(|u| u.view_mode = ViewMode::Canvas)
                        title="Board canvas view"
                    >
                        "Board"
                    </button>
                    <button
                        class="btn toolbar__segment-btn"
                        class:toolbar__segment-btn--active=move || ui.get().view_mode == ViewMode::Trace
                        on:click=move |_| ui.update(|u| u.view_mode = ViewMode::Trace)
                        title="Trace view"
                    >
                        "Traces"
                    </button>
                </div>
            </Show>

            <Show when=move || location.pathname.get().starts_with("/board/")>
                <button class="btn toolbar__share" on:click=on_share title="Share board">
                    "Share"
                </button>
            </Show>
            <Show when=move || location.pathname.get().starts_with("/board/") && board.get().board_id.is_some()>
                <a class="btn toolbar__share" href=export_href title="Export board as JSONL snapshot">
                    "Export"
                </a>
            </Show>

            <span class="toolbar__spacer"></span>

            <span class="toolbar__self">
                {move || self_identity().0}
                " ("
                <span class="toolbar__self-method">{move || self_identity().1}</span>
                ")"
            </span>

            <button class="btn toolbar__logout" on:click=on_logout title="Logout">
                "Logout"
            </button>
        </div>

        <Show when=move || show_share.get()>
            <ShareDialog board=board on_cancel=on_share_cancel />
        </Show>
    }
}

/// Dialog for generating and displaying a board access code.
#[component]
fn ShareDialog(board: RwSignal<BoardState>, on_cancel: Callback<()>) -> impl IntoView {
    let sender = expect_context::<RwSignal<FrameSender>>();

    let on_generate = move |_| {
        let Some(board_id) = board.get_untracked().board_id else {
            return;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id),
            from: None,
            syscall: "board:access:generate".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({}),
        };
        let _ = sender.get_untracked().send(&frame);
    };

    view! {
        <div class="dialog-backdrop" on:click=move |_| on_cancel.run(())>
            <div class="dialog" on:click=move |ev| ev.stop_propagation()>
                <h2>"Share Board"</h2>
                <Show
                    when=move || board.get().generated_access_code.is_some()
                    fallback=move || {
                        view! {
                            <p class="dialog__hint">"Generate a 6-character access code to share this board with others."</p>
                            <div class="dialog__actions">
                                <button class="btn" on:click=move |_| on_cancel.run(())>
                                    "Cancel"
                                </button>
                                <button class="btn btn--primary" on:click=on_generate>
                                    "Generate"
                                </button>
                            </div>
                        }
                    }
                >
                    <p class="dialog__hint">"Share this access code:"</p>
                    <p class="dialog__code">
                        {move || board.get().generated_access_code.unwrap_or_default()}
                    </p>
                    <div class="dialog__actions">
                        <button class="btn btn--primary" on:click=move |_| on_cancel.run(())>
                            "Done"
                        </button>
                    </div>
                </Show>
            </div>
        </div>
    }
}
