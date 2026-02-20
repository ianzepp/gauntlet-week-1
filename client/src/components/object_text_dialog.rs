//! Modal dialog for editing selected object text content.

use leptos::prelude::*;

/// Dialog shown when object text editing is active.
#[component]
pub fn ObjectTextDialog(
    value: RwSignal<String>,
    on_cancel: Callback<()>,
    on_save: Callback<()>,
    on_keydown: Callback<leptos::ev::KeyboardEvent>,
) -> impl IntoView {
    view! {
        <div class="dialog-backdrop" on:click=move |_| on_cancel.run(())>
            <div
                class="dialog dialog--object-text"
                on:click=move |ev| ev.stop_propagation()
                on:keydown=move |ev| on_keydown.run(ev)
            >
                <label class="dialog__label">
                    "Text"
                    <textarea
                        class="dialog__textarea"
                        prop:value=move || value.get()
                        on:input=move |ev| value.set(event_target_value(&ev))
                        on:keydown=move |ev| on_keydown.run(ev)
                        autofocus=true
                    ></textarea>
                </label>
                <div class="dialog__actions">
                    <button class="btn" on:click=move |_| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button class="btn btn--primary" on:click=move |_| on_save.run(())>
                        "Save"
                    </button>
                </div>
            </div>
        </div>
    }
}
