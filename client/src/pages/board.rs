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
use leptos::tachys::view::any_view::IntoAny;
use leptos_router::hooks::use_params_map;

use crate::app::FrameSender;
use crate::components::board_stamp::BoardStamp;
use crate::components::canvas_host::CanvasHost;
use crate::components::help_shortcuts_modal::HelpShortcutsModal;
use crate::components::left_panel::LeftPanel;
use crate::components::object_text_dialog::ObjectTextDialog;
use crate::components::right_panel::RightPanel;
use crate::components::status_bar::StatusBar;
use crate::components::toolbar::Toolbar;
use crate::components::trace_view::TraceView;
use crate::pages::board_prompt::assistant_preview_and_has_more;
use crate::pages::board_prompt_bar::{BoardPromptBar, PromptBarStatus};
use crate::state::ai::{AiMessage, AiState};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::ui::{RightTab, UiState, ViewMode};
use crate::util::auth::install_unauth_redirect;
use crate::util::frame::request_frame;

fn build_board_membership_frame(syscall: &str, board_id: String) -> crate::net::types::Frame {
    request_frame(syscall, Some(board_id), serde_json::json!({}))
}

fn reset_board_for_route_change(board: &mut BoardState, next_board_id: Option<String>) {
    board.board_id = next_board_id;
    board.board_name = None;
    board.is_public = false;
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
    let ai = expect_context::<RwSignal<AiState>>();
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let params = use_params_map();
    let last_join_key = RwSignal::new(None::<(String, String)>);
    let last_route_board_id = RwSignal::new(None::<String>);
    let prompt_input = RwSignal::new(String::new());
    let prompt_status = RwSignal::new(PromptBarStatus::Idle);
    let pending_message_start = RwSignal::new(None::<usize>);
    let prompt_preview_user = RwSignal::new(String::new());
    let prompt_preview_assistant = RwSignal::new(String::new());
    let prompt_preview_assistant_has_more = RwSignal::new(false);
    let prompt_preview_assistant_error = RwSignal::new(false);
    let object_text_dialog_open = RwSignal::new(false);
    let object_text_dialog_id = RwSignal::new(None::<String>);
    let object_text_dialog_value = RwSignal::new(String::new());
    let last_object_text_dialog_seq = RwSignal::new(0_u64);
    let help_modal_open = RwSignal::new(false);

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
            b.is_public = false;
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

    let navigate = leptos_router::hooks::use_navigate();
    install_unauth_redirect(auth, navigate);

    Effect::new(move || {
        let Some(start_idx) = pending_message_start.get() else {
            return;
        };
        let ai_state = ai.get();
        if ai_state.loading {
            return;
        }
        let has_error = ai_state
            .messages
            .iter()
            .skip(start_idx)
            .any(|msg| msg.role == "error");
        if let Some(reply) = ai_state
            .messages
            .iter()
            .skip(start_idx)
            .rev()
            .find(|msg| msg.role == "assistant" || msg.role == "error")
        {
            let (preview, has_more) = assistant_preview_and_has_more(&reply.content);
            prompt_preview_assistant.set(preview);
            prompt_preview_assistant_has_more.set(has_more);
            prompt_preview_assistant_error.set(reply.role == "error");
        }
        prompt_status.set(if has_error {
            PromptBarStatus::Error
        } else {
            PromptBarStatus::Success
        });
        pending_message_start.set(None);
    });

    Effect::new(move || {
        let seq = ui.get().object_text_dialog_seq;
        if seq == last_object_text_dialog_seq.get_untracked() {
            return;
        }
        last_object_text_dialog_seq.set(seq);

        let state = board.get();
        let Some(id) = state.selection.iter().next().cloned() else {
            return;
        };
        let Some(obj) = state.objects.get(&id) else {
            return;
        };
        let text = obj
            .props
            .get("text")
            .or_else(|| obj.props.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        object_text_dialog_id.set(Some(id));
        object_text_dialog_value.set(text);
        object_text_dialog_open.set(true);
    });

    Effect::new(move || {
        if !object_text_dialog_open.get() {
            return;
        }
        let Some(target_id) = object_text_dialog_id.get() else {
            object_text_dialog_open.set(false);
            return;
        };
        let state = board.get();
        let still_selected = state.selection.contains(&target_id);
        let still_exists = state.objects.contains_key(&target_id);
        if !still_selected || !still_exists {
            object_text_dialog_open.set(false);
            object_text_dialog_id.set(None);
        }
    });

    let send_prompt = move || {
        let text = prompt_input.get();
        if text.trim().is_empty() || ai.get().loading {
            return;
        }

        let prompt = text.trim().to_owned();
        let frame = request_frame(
            "ai:prompt",
            board.get().board_id.clone(),
            serde_json::json!({ "prompt": prompt }),
        );
        let frame_id = frame.id.clone();

        prompt_status.set(PromptBarStatus::Loading);
        prompt_preview_user.set(prompt.clone());
        prompt_preview_assistant.set(String::new());
        prompt_preview_assistant_has_more.set(false);
        prompt_preview_assistant_error.set(false);
        if sender.get().send(&frame) {
            let start_idx = ai.get_untracked().messages.len();
            ai.update(|a| {
                a.messages.push(AiMessage {
                    id: frame_id,
                    role: "user".to_owned(),
                    content: prompt,
                    timestamp: 0.0,
                    mutations: None,
                });
                a.loading = true;
            });
            pending_message_start.set(Some(start_idx));
            prompt_input.set(String::new());
        } else {
            ai.update(|a| {
                a.messages.push(AiMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: "error".to_owned(),
                    content: "AI request failed: not connected".to_owned(),
                    timestamp: 0.0,
                    mutations: None,
                });
                a.loading = false;
            });
            prompt_preview_assistant.set("AI request failed: not connected".to_owned());
            prompt_preview_assistant_has_more.set(false);
            prompt_preview_assistant_error.set(true);
            prompt_status.set(PromptBarStatus::Error);
            pending_message_start.set(None);
        }
    };

    let on_prompt_submit = Callback::new(move |()| send_prompt());

    let on_prompt_read_more = Callback::new(move |()| {
        ui.update(|u| {
            u.right_panel_expanded = true;
            u.right_tab = RightTab::Ai;
            u.ai_focus_seq = u.ai_focus_seq.saturating_add(1);
        });
    });

    let on_object_text_cancel = Callback::new(move |()| {
        object_text_dialog_open.set(false);
        object_text_dialog_id.set(None);
    });
    let on_object_text_keydown = Callback::new(move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            object_text_dialog_open.set(false);
            object_text_dialog_id.set(None);
        }
    });

    let on_object_text_save = Callback::new(move |()| {
        let Some(id) = object_text_dialog_id.get() else {
            object_text_dialog_open.set(false);
            return;
        };
        let value = object_text_dialog_value.get();
        let Some(obj) = board.get().objects.get(&id).cloned() else {
            object_text_dialog_open.set(false);
            object_text_dialog_id.set(None);
            return;
        };

        let mut props = obj.props.as_object().cloned().unwrap_or_default();
        props.insert("text".to_owned(), serde_json::json!(value.clone()));
        props.insert("content".to_owned(), serde_json::json!(value));
        let next_props = serde_json::Value::Object(props);

        board.update(|b| {
            if let Some(existing) = b.objects.get_mut(&id) {
                existing.props = next_props.clone();
            }
        });

        sender.get().send(&request_frame(
            "object:update",
            Some(obj.board_id),
            serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": next_props,
            }),
        ));

        object_text_dialog_open.set(false);
        object_text_dialog_id.set(None);
    });

    let on_help_open = Callback::new(move |()| help_modal_open.set(true));
    let on_help_close = Callback::new(move |()| help_modal_open.set(false));
    let on_board_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let key = ev.key();
        if key == "?" || (key == "/" && ev.shift_key()) || (key == "Escape" && help_modal_open.get_untracked()) {
            #[cfg(feature = "hydrate")]
            {
                if is_text_input_target(&ev) {
                    return;
                }
            }
            ev.prevent_default();
            if key == "Escape" {
                help_modal_open.set(false);
            } else {
                help_modal_open.set(true);
            }
        }
    };

    view! {
        <div
            class="board-page"
            class:board-page--left-expanded=move || ui.get().left_panel_expanded
            class:board-page--right-expanded=move || ui.get().right_panel_expanded
            class:board-page--trace=move || ui.get().view_mode == ViewMode::Trace
            on:keydown=on_board_keydown
        >
            <div class="board-page__toolbar">
                <Toolbar/>
            </div>
            {move || {
                if ui.get().view_mode == ViewMode::Canvas {
                    view! {
                        <div class="board-page__left-panel">
                            <LeftPanel/>
                        </div>
                    }
                        .into_any()
                } else {
                    let _: () = view! { <></> };
                    ().into_any()
                }
            }}
            <div class="board-page__canvas">
                {move || {
                    if ui.get().view_mode == ViewMode::Canvas {
                        view! {
                            <CanvasHost/>
                            <BoardStamp/>
                            <div class="board-page__input-overlay">
                                <BoardPromptBar
                                    prompt_input=prompt_input
                                    prompt_status=prompt_status
                                    prompt_preview_user=prompt_preview_user
                                    prompt_preview_assistant=prompt_preview_assistant
                                    prompt_preview_assistant_has_more=prompt_preview_assistant_has_more
                                    prompt_preview_assistant_error=prompt_preview_assistant_error
                                    on_submit=on_prompt_submit
                                    on_read_more=on_prompt_read_more
                                />
                            </div>
                        }
                            .into_any()
                    } else {
                        view! { <TraceView/> }.into_any()
                    }
                }}
            </div>
            <div class="board-page__right-panel">
                <RightPanel/>
            </div>
            <div class="board-page__status-bar">
                <StatusBar on_help=on_help_open/>
            </div>
            {move || {
                if object_text_dialog_open.get() {
                    view! {
                        <ObjectTextDialog
                            value=object_text_dialog_value
                            on_cancel=on_object_text_cancel
                            on_save=on_object_text_save
                            on_keydown=on_object_text_keydown
                        />
                    }
                        .into_any()
                } else {
                    let _: () = view! { <></> };
                    ().into_any()
                }
            }}
            <Show when=move || help_modal_open.get()>
                <HelpShortcutsModal on_close=on_help_close />
            </Show>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn is_text_input_target(ev: &leptos::ev::KeyboardEvent) -> bool {
    use wasm_bindgen::JsCast;

    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };
    let tag = element.tag_name().to_ascii_lowercase();
    if tag == "input" || tag == "textarea" || tag == "select" {
        return true;
    }
    element.has_attribute("contenteditable")
}

#[cfg(test)]
#[path = "board_test.rs"]
mod board_test;
