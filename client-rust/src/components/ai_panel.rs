//! AI assistant panel for sending prompts and displaying responses.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};
use crate::state::ai::AiState;
use crate::state::board::BoardState;

/// AI panel showing conversation history and a prompt input.
#[component]
pub fn AiPanel() -> impl IntoView {
    let ai = expect_context::<RwSignal<AiState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let input = RwSignal::new(String::new());
    let last_history_board = RwSignal::new(None::<String>);
    let messages_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move || {
        let Some(board_id) = board.get().board_id else {
            return;
        };

        if last_history_board.get().as_deref() == Some(board_id.as_str()) {
            return;
        }

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0.0,
            board_id: Some(board_id.clone()),
            from: None,
            syscall: "ai:history".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({}),
        };
        sender.get().send(&frame);
        last_history_board.set(Some(board_id));
    });

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

    let do_send = move || {
        let text = input.get();
        if text.trim().is_empty() || ai.get().loading {
            return;
        }

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0.0,
            board_id: board.get().board_id.clone(),
            from: None,
            syscall: "ai:prompt".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({ "prompt": text }),
        };
        sender.get().send(&frame);
        ai.update(|a| a.loading = true);
        input.set(String::new());
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
                                    <span class="ai-panel__role">{role.clone()}</span>
                                    <div class="ai-panel__content" class:ai-panel__markdown=is_assistant>
                                        {if is_assistant {
                                            view! { <pre class="ai-panel__markdown-pre">{content}</pre> }.into_any()
                                        } else {
                                            view! { <span>{content}</span> }.into_any()
                                        }}
                                    </div>
                                    {mutation_count
                                        .filter(|count| *count > 0)
                                        .map(|count| {
                                            view! {
                                                <span class="ai-panel__mutations">{format!("{} objects modified", count)}</span>
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
