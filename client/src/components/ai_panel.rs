//! AI assistant panel for sending prompts and displaying responses.
//!
//! SYSTEM CONTEXT
//! ==============
//! Encodes prompt interactions as `ai:prompt` frames and renders streamed
//! response history from shared AI state.

use leptos::prelude::*;
use pulldown_cmark::{Event, Options, Parser, html};

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};
use crate::state::ai::AiState;
use crate::state::board::BoardState;
use crate::state::ui::{RightTab, UiState};

/// AI panel showing conversation history and a prompt input.
#[component]
pub fn AiPanel() -> impl IntoView {
    let ai = expect_context::<RwSignal<AiState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let ui = expect_context::<RwSignal<UiState>>();

    let input = RwSignal::new(String::new());
    let messages_ref = NodeRef::<leptos::html::Div>::new();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    Effect::new(move || {
        let state = ai.get();
        let _ = state.messages.len();
        let _ = state.loading;

        #[cfg(feature = "hydrate")]
        {
            if let Some(el) = messages_ref.get() {
                let scroll_height = el.scroll_height();
                el.set_scroll_top(scroll_height);
            }
        }
    });

    Effect::new(move || {
        let seq = ui.get().ai_focus_seq;
        let ui_state = ui.get();
        let _ = seq;
        if ui_state.right_panel_expanded && ui_state.right_tab == RightTab::Ai {
            #[cfg(feature = "hydrate")]
            {
                if let Some(input_el) = input_ref.get() {
                    let _ = input_el.focus();
                }
            }
        }
    });

    let do_send = move || {
        let text = input.get();
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
            trace: None,
            data: serde_json::json!({ "prompt": prompt }),
        };
        if sender.get().send(&frame) {
            ai.update(|a| {
                a.messages.push(crate::state::ai::AiMessage {
                    id: frame_id,
                    role: "user".to_owned(),
                    content: prompt,
                    timestamp: 0.0,
                    mutations: None,
                });
                a.loading = true;
            });
            input.set(String::new());
        } else {
            ai.update(|a| {
                a.messages.push(crate::state::ai::AiMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: "error".to_owned(),
                    content: "AI request failed: not connected".to_owned(),
                    timestamp: 0.0,
                    mutations: None,
                });
                a.loading = false;
            });
        }
    };

    let on_click = move |_| do_send();

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send();
        }
    };

    let can_send = move || !input.get().trim().is_empty() && !ai.get().loading;

    view! {
        <div class="ai-panel">
            <div class="ai-panel__messages" node_ref=messages_ref>
                {move || {
                    let messages = ai.get().messages;
                    if messages.is_empty() {
                        return view! {
                            <div class="ai-panel__empty">"No notes yet"</div>
                        }
                            .into_any();
                    }

                    messages
                        .iter()
                        .map(|msg| {
                            let role = msg.role.clone();
                            let content = msg.content.clone();
                            let is_assistant = role == "assistant";
                            let is_error = role == "error";
                            let mutation_count = msg.mutations;

                            view! {
                                <div
                                    class="ai-panel__message"
                                    class:ai-panel__message--assistant=is_assistant
                                    class:ai-panel__message--error=is_error
                                >
                                    <div class="ai-panel__content" class:ai-panel__markdown=is_assistant>
                                        {if is_assistant {
                                            let rendered = render_markdown_html(&content);
                                            view! {
                                                <div class="ai-panel__markdown-body" inner_html=rendered></div>
                                            }
                                                .into_any()
                                        } else {
                                            view! { <span>{content}</span> }.into_any()
                                        }}
                                    </div>
                                    {mutation_count
                                        .filter(|count| *count > 0)
                                        .map(|count| {
                                            view! {
                                                <span class="ai-panel__mutations">{format!("{count} objects modified")}</span>
                                            }
                                        })}
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                }}

                {move || {
                    ai.get()
                        .loading
                        .then(|| view! { <div class="ai-panel__loading">"Thinking..."</div> })
                }}
            </div>

            <div class="ai-panel__input-row">
                <input
                    class="ai-panel__input"
                    type="text"
                    placeholder="Ask the AI..."
                    node_ref=input_ref
                    disabled=move || ai.get().loading
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=on_keydown
                />
                <button class="btn btn--primary ai-panel__send" on:click=on_click disabled=move || !can_send()>
                    "Send"
                </button>
            </div>
        </div>
    }
}

fn render_markdown_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    // Safety: drop inline/block raw HTML from model output before rendering.
    let parser = Parser::new_ext(markdown, options).filter_map(|event| match event {
        Event::Html(_) | Event::InlineHtml(_) => None,
        other => Some(other),
    });

    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}
