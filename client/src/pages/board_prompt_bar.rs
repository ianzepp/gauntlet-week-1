//! Board prompt input and inline preview UI.

#[cfg(test)]
#[path = "board_prompt_bar_test.rs"]
mod board_prompt_bar_test;

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum PromptBarStatus {
    #[default]
    Idle,
    Loading,
    Success,
    Error,
}

#[component]
pub(crate) fn BoardPromptBar(
    prompt_input: RwSignal<String>,
    prompt_status: RwSignal<PromptBarStatus>,
    prompt_preview_user: RwSignal<String>,
    prompt_preview_assistant: RwSignal<String>,
    prompt_preview_assistant_has_more: RwSignal<bool>,
    prompt_preview_assistant_error: RwSignal<bool>,
    on_submit: Callback<()>,
    on_read_more: Callback<()>,
) -> impl IntoView {
    let on_prompt_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            on_submit.run(());
        }
    };

    let on_prompt_focus = move |_| {
        if prompt_status.get_untracked() != PromptBarStatus::Loading {
            prompt_status.set(PromptBarStatus::Idle);
        }
    };

    view! {
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
                            <button class="board-page__prompt-preview-more" on:click=move |_| on_read_more.run(())>
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
    }
}
