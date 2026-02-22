//! AI assistant panel for sending prompts and displaying responses.
//!
//! SYSTEM CONTEXT
//! ==============
//! Encodes prompt interactions as `ai:prompt` frames and renders streamed
//! response history from shared AI state.

use leptos::prelude::*;
use pulldown_cmark::{Event, Options, Parser, html};

use crate::state::ai::AiState;

/// AI panel showing conversation history and a prompt input.
#[component]
pub fn AiPanel() -> impl IntoView {
    let ai = expect_context::<RwSignal<AiState>>();
    let messages_ref = NodeRef::<leptos::html::Div>::new();

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
