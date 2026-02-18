//! AI assistant panel for sending prompts and displaying responses.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};
use crate::state::ai::AiState;
use crate::state::board::BoardState;

/// AI panel showing conversation history and a prompt input.
///
/// Sends `ai:prompt` frames to the server and displays responses.
#[component]
pub fn AiPanel() -> impl IntoView {
    let ai = expect_context::<RwSignal<AiState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let input = RwSignal::new(String::new());

    let do_send = move || {
        let text = input.get();
        if text.trim().is_empty() {
            return;
        }
        let board_id = board.get().board_id.clone();
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0.0,
            board_id,
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

    view! {
        <div class="ai-panel">
            <div class="ai-panel__messages">
                {move || {
                    ai.get()
                        .messages
                        .iter()
                        .map(|msg| {
                            let role = msg.role.clone();
                            let content = msg.content.clone();
                            let is_assistant = role == "assistant";
                            view! {
                                <div
                                    class="ai-panel__message"
                                    class:ai-panel__message--assistant=is_assistant
                                >
                                    <span class="ai-panel__role">{role}</span>
                                    <div class="ai-panel__content">{content}</div>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
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
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=on_keydown
                />
                <button class="btn btn--primary" on:click=on_click>
                    "Send"
                </button>
            </div>
        </div>
    }
}
