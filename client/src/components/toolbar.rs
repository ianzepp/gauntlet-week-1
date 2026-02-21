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
use crate::state::boards::BoardsState;
use crate::state::ui::{UiState, ViewMode};
use crate::util::frame::request_frame;

/// Top toolbar for the board page.
#[component]
pub fn Toolbar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let boards = expect_context::<RwSignal<BoardsState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let location = use_location();
    let navigate = use_navigate();
    let import_file_ref = NodeRef::<leptos::html::Input>::new();

    let show_share = RwSignal::new(false);

    let board_name = move || {
        board
            .get()
            .board_name
            .unwrap_or_else(|| "Untitled".to_owned())
    };
    let on_export = move |_| {
        #[cfg(feature = "hydrate")]
        {
            let export_href = board
                .get()
                .board_id
                .map(|id| format!("/api/boards/{id}/export.jsonl"))
                .unwrap_or_else(|| "#".to_owned());
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href(&export_href);
            }
        }
    };
    let on_import_click = {
        move |_| {
            #[cfg(feature = "hydrate")]
            {
                use wasm_bindgen::JsCast;
                if let Some(input) = import_file_ref.get()
                    && let Some(element) = input.dyn_ref::<web_sys::HtmlElement>()
                {
                    input.set_value("");
                    element.click();
                }
            }
        }
    };
    let on_import_change = {
        move |_ev: leptos::ev::Event| {
            #[cfg(feature = "hydrate")]
            {
                use wasm_bindgen::JsCast;

                let sender = sender;
                let board = board;
                leptos::task::spawn_local(async move {
                    let Some(input) = _ev
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    else {
                        return;
                    };
                    let Some(files) = input.files() else {
                        return;
                    };
                    let Some(file) = files.get(0) else {
                        return;
                    };
                    let Ok(text_value) = wasm_bindgen_futures::JsFuture::from(file.text()).await else {
                        return;
                    };
                    let Some(jsonl) = text_value.as_string() else {
                        return;
                    };
                    let Some(board_id) = board.get_untracked().board_id else {
                        return;
                    };
                    let url = format!("/api/boards/{board_id}/import.jsonl");
                    let body = serde_json::json!({ "jsonl": jsonl }).to_string();
                    let Ok(request) = gloo_net::http::Request::post(&url)
                        .header("Content-Type", "application/json")
                        .body(body)
                    else {
                        return;
                    };
                    let Ok(response) = request.send().await else {
                        return;
                    };
                    if !response.ok() {
                        return;
                    }
                    let _ = sender
                        .get_untracked()
                        .send(&crate::util::frame::request_frame(
                            "board:join",
                            Some(board_id),
                            serde_json::json!({}),
                        ));
                });
            }
        }
    };

    let self_identity = move || {
        auth.get()
            .user
            .map_or_else(|| ("me".to_owned(), "session".to_owned()), |user| (user.name, user.auth_method))
    };
    let can_toggle_visibility = move || {
        let Some(board_id) = board.get().board_id else {
            return false;
        };
        let Some(user_id) = auth.get().user.map(|u| u.id) else {
            return false;
        };
        boards
            .get()
            .items
            .iter()
            .find(|item| item.id == board_id)
            .and_then(|item| item.owner_id.as_deref())
            == Some(user_id.as_str())
    };
    let set_visibility = Callback::new(move |is_public: bool| {
        let Some(board_id) = board.get().board_id else {
            return;
        };
        let frame = request_frame(
            "board:visibility:set",
            Some(board_id.clone()),
            serde_json::json!({
                "board_id": board_id,
                "is_public": is_public
            }),
        );
        let _ = sender.get().send(&frame);
        board.update(|b| b.is_public = is_public);
        boards.update(|s| {
            if let Some(item) = s.items.iter_mut().find(|item| item.id == board_id) {
                item.is_public = is_public;
            }
        });
    });

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

    let on_share_cancel = Callback::new(move |()| show_share.set(false));
    let on_back = Callback::new(move |()| {
        navigate("/", leptos_router::NavigateOptions::default());
    });

    let set_visibility_public = set_visibility;
    let set_visibility_private = set_visibility;

    view! {
        <div class="toolbar">
            <input
                type="file"
                accept=".jsonl,.ndjson,application/x-ndjson,application/json"
                node_ref=import_file_ref
                style="display:none"
                on:change=on_import_change
            />
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
            <Show when=move || location.pathname.get().starts_with("/board/") && can_toggle_visibility()>
                <div class="toolbar__segment" role="group" aria-label="Board visibility mode">
                    <button
                        class="btn toolbar__segment-btn"
                        class:toolbar__segment-btn--active=move || board.get().is_public
                        on:click=move |_| set_visibility_public.run(true)
                        title="Visible to all users"
                    >
                        "Public"
                    </button>
                    <button
                        class="btn toolbar__segment-btn"
                        class:toolbar__segment-btn--active=move || !board.get().is_public
                        on:click=move |_| set_visibility_private.run(false)
                        title="Visible only to members"
                    >
                        "Private"
                    </button>
                </div>
            </Show>

            <Show when=move || location.pathname.get().starts_with("/board/")>
                <button class="btn toolbar__share" on:click=on_share title="Share board">
                    "Share"
                </button>
            </Show>
            <Show when=move || location.pathname.get().starts_with("/board/") && board.get().board_id.is_some()>
                <button class="btn toolbar__share" on:click=on_import_click title="Import board snapshot from JSONL">
                    "Import"
                </button>
            </Show>
            <Show when=move || location.pathname.get().starts_with("/board/") && board.get().board_id.is_some()>
                <button class="btn toolbar__share" on:click=on_export title="Export board as JSONL snapshot">
                    "Export"
                </button>
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
