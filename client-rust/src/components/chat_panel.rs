//! Real-time board chat panel displaying and sending messages.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::chat::ChatState;

/// Chat panel showing message history and an input for sending new messages.
#[component]
pub fn ChatPanel() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let chat = expect_context::<RwSignal<ChatState>>();
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
            ts: 0,
            board_id: Some(board_id.clone()),
            from: None,
            syscall: "chat:history".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({}),
        };
        sender.get().send(&frame);
        last_history_board.set(Some(board_id));
    });

    Effect::new(move || {
        let _ = chat.get().messages.len();

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
        if text.trim().is_empty() {
            return;
        }

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: board.get().board_id.clone(),
            from: None,
            syscall: "chat:message".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({ "message": text }),
        };
        sender.get().send(&frame);
        input.set(String::new());
    };

    let on_click = move |_| do_send();

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send();
        }
    };

    let placeholder = move || {
        let username = auth
            .get()
            .user
            .map(|u| u.name)
            .unwrap_or_else(|| "unknown".to_owned());
        format!("Message as {username}...")
    };

    let can_send = move || !input.get().trim().is_empty();

    view! {
        <div class="chat-panel">
            <div class="chat-panel__messages" node_ref=messages_ref>
                {move || {
                    let messages = chat.get().messages;
                    if messages.is_empty() {
                        return view! {
                            <div class="chat-panel__empty">"No messages yet"</div>
                        }
                            .into_any();
                    }

                    messages
                        .iter()
                        .map(|msg| {
                            let color = msg.user_color.clone();
                            let name = msg.user_name.clone();
                            let content = msg.content.clone();
                            view! {
                                <div class="chat-panel__message">
                                    <span class="chat-panel__author" style:color=color>
                                        {name}
                                    </span>
                                    <span class="chat-panel__text">{content}</span>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                }}
            </div>

            <div class="chat-panel__input-row">
                <input
                    class="chat-panel__input"
                    type="text"
                    placeholder=placeholder
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=on_keydown
                />
                <button class="btn btn--primary chat-panel__send" on:click=on_click disabled=move || !can_send()>
                    "Send"
                </button>
            </div>
        </div>
    }
}
