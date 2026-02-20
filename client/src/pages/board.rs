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
use crate::state::ai::{AiMessage, AiState};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::ui::{RightTab, UiState, ViewMode};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum PromptBarStatus {
    #[default]
    Idle,
    Loading,
    Success,
    Error,
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
        if let Some(reply) = ai_state.messages.iter().skip(start_idx).rev().find(|msg| {
            msg.role == "assistant" || msg.role == "error"
        }) {
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
        let frame_id = uuid::Uuid::new_v4().to_string();
        let frame = Frame {
            id: frame_id.clone(),
            parent_id: None,
            ts: 0,
            board_id: board.get().board_id.clone(),
            from: None,
            syscall: "ai:prompt".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({ "prompt": prompt }),
        };

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

    let on_prompt_focus = move |_| {
        if prompt_status.get_untracked() != PromptBarStatus::Loading {
            prompt_status.set(PromptBarStatus::Idle);
        }
    };

    let on_prompt_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            send_prompt();
        }
    };

    let on_prompt_read_more = move |_| {
        ui.update(|u| {
            u.right_panel_expanded = true;
            u.right_tab = RightTab::Ai;
            u.ai_focus_seq = u.ai_focus_seq.saturating_add(1);
        });
    };

    let on_object_text_cancel = move |_| {
        object_text_dialog_open.set(false);
        object_text_dialog_id.set(None);
    };
    let on_object_text_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            ev.prevent_default();
            object_text_dialog_open.set(false);
            object_text_dialog_id.set(None);
        }
    };

    let on_object_text_save = move |_| {
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

        sender.get().send(&Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": next_props,
            }),
        });

        object_text_dialog_open.set(false);
        object_text_dialog_id.set(None);
    };

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
                    <div class="board-page__input-overlay">
                        <div class="board-page__prompt-bar">
                            <div
                                class="board-page__prompt-preview"
                                class:board-page__prompt-preview--empty=move || {
                                    prompt_preview_user.get().is_empty() && prompt_preview_assistant.get().is_empty()
                                }
                            >
                                <div
                                    class="board-page__prompt-preview-row board-page__prompt-preview-row--user"
                                    class:board-page__prompt-preview-row--empty=move || prompt_preview_user.get().is_empty()
                                >
                                    <span class="board-page__prompt-preview-text">{move || prompt_preview_user.get()}</span>
                                </div>
                                <div
                                    class="board-page__prompt-preview-row board-page__prompt-preview-row--assistant"
                                    class:board-page__prompt-preview-row--empty=move || prompt_preview_assistant.get().is_empty()
                                    class:board-page__prompt-preview-row--error=move || prompt_preview_assistant_error.get()
                                >
                                    <span class="board-page__prompt-preview-text">
                                        {move || prompt_preview_assistant.get()}
                                        <Show when=move || prompt_preview_assistant_has_more.get() && !prompt_preview_assistant_error.get()>
                                            <button class="board-page__prompt-preview-more" on:click=on_prompt_read_more>
                                                "[more]"
                                            </button>
                                        </Show>
                                    </span>
                                </div>
                            </div>
                            <div class="board-page__input-row">
                                <input
                                    class="board-page__input-line"
                                    type="text"
                                    placeholder="Ask the AI..."
                                    prop:value=move || prompt_input.get()
                                    on:input=move |ev| prompt_input.set(event_target_value(&ev))
                                    on:focus=on_prompt_focus
                                    on:keydown=on_prompt_keydown
                                />
                                <div class="board-page__prompt-status" aria-live="polite">
                                    {move || match prompt_status.get() {
                                        PromptBarStatus::Idle => view! { <span class="board-page__prompt-icon-spacer"></span> }.into_any(),
                                        PromptBarStatus::Loading => view! { <span class="board-page__prompt-spinner"></span> }.into_any(),
                                        PromptBarStatus::Success => view! {
                                            <svg class="board-page__prompt-icon board-page__prompt-icon--success" viewBox="0 0 20 20" aria-hidden="true">
                                                <path d="M4 10.5 8 14.5 16 6.5"></path>
                                            </svg>
                                        }.into_any(),
                                        PromptBarStatus::Error => view! {
                                            <svg class="board-page__prompt-icon board-page__prompt-icon--error" viewBox="0 0 20 20" aria-hidden="true">
                                                <path d="M5.5 5.5 14.5 14.5"></path>
                                                <path d="M14.5 5.5 5.5 14.5"></path>
                                            </svg>
                                        }.into_any(),
                                    }}
                                </div>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
            <div class="board-page__right-panel">
                <RightPanel/>
            </div>
            <div class="board-page__status-bar">
                <StatusBar/>
            </div>
            <Show when=move || object_text_dialog_open.get()>
                <div class="dialog-backdrop" on:click=on_object_text_cancel>
                    <div class="dialog dialog--object-text" on:click=move |ev| ev.stop_propagation() on:keydown=on_object_text_keydown>
                        <label class="dialog__label">
                            "Text"
                            <textarea
                                class="dialog__textarea"
                                prop:value=move || object_text_dialog_value.get()
                                on:input=move |ev| object_text_dialog_value.set(event_target_value(&ev))
                                on:keydown=on_object_text_keydown
                                autofocus=true
                            ></textarea>
                        </label>
                        <div class="dialog__actions">
                            <button class="btn" on:click=on_object_text_cancel>
                                "Cancel"
                            </button>
                            <button class="btn btn--primary" on:click=on_object_text_save>
                                "Save"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

fn assistant_preview_and_has_more(text: &str) -> (String, bool) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (String::new(), false);
    }

    let paragraphs = split_paragraphs(trimmed);
    let mut preview: Vec<String> = Vec::new();
    let mut has_more = false;

    for para in paragraphs.iter() {
        if paragraph_is_structured(para) {
            if para.trim_end().ends_with(':') && preview.len() < 3 {
                preview.push(para.clone());
            }
            has_more = true;
            break;
        }

        if preview.len() < 3 {
            preview.push(para.clone());
        } else {
            has_more = true;
            break;
        }
    }

    if preview.is_empty() {
        if let Some(first) = paragraphs.first() {
            preview.push(first.clone());
        }
    }

    if !has_more && paragraphs.len() > preview.len() {
        has_more = true;
    }

    (preview.join("\n\n"), has_more)
}

fn split_paragraphs(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                out.push(current.join("\n").trim().to_owned());
                current.clear();
            }
            continue;
        }
        current.push(line.trim_end());
    }
    if !current.is_empty() {
        out.push(current.join("\n").trim().to_owned());
    }
    out.into_iter().filter(|p| !p.is_empty()).collect()
}

fn paragraph_is_structured(para: &str) -> bool {
    let trimmed = para.trim();
    if trimmed.ends_with(':') {
        return true;
    }
    para.lines().any(line_is_structured)
}

fn line_is_structured(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        return true;
    }
    if starts_with_markdown_numbered_list(t) {
        return true;
    }
    if t.starts_with('|') {
        return true;
    }
    t.contains('|') && (t.contains("---") || t.contains(":---") || t.contains("---:"))
}

fn starts_with_markdown_numbered_list(text: &str) -> bool {
    let mut saw_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if (ch == '.' || ch == ')') && saw_digit {
            return text
                .chars()
                .skip_while(|c| c.is_ascii_digit())
                .nth(1)
                .is_some_and(char::is_whitespace);
        }
        break;
    }
    false
}

#[cfg(test)]
#[path = "board_test.rs"]
mod board_test;
