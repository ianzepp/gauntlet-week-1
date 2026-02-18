//! Collapsible right panel with tab switcher for Chat, AI, and Boards.

use leptos::prelude::*;

use crate::components::ai_panel::AiPanel;
use crate::components::chat_panel::ChatPanel;
use crate::components::mission_control::MissionControl;
use crate::state::ui::{RightTab, UiState};

/// Collapsible right sidebar with tab switching between Chat, AI, and Boards.
#[component]
pub fn RightPanel() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let expanded = move || ui.get().right_panel_expanded;
    let active_tab = move || ui.get().right_tab;

    let toggle_expand = move |_| {
        ui.update(|u| u.right_panel_expanded = !u.right_panel_expanded);
    };

    let set_chat = move |_| ui.update(|u| u.right_tab = RightTab::Chat);
    let set_ai = move |_| ui.update(|u| u.right_tab = RightTab::Ai);
    let set_boards = move |_| ui.update(|u| u.right_tab = RightTab::Boards);

    view! {
        <div class="right-panel" class:right-panel--collapsed=move || !expanded()>
            <div class="right-panel__tabs">
                <button class="right-panel__toggle" on:click=toggle_expand>
                    {move || if expanded() { "\u{25B6}" } else { "\u{25C0}" }}
                </button>
                <button
                    class="right-panel__tab"
                    class:right-panel__tab--active=move || active_tab() == RightTab::Chat
                    on:click=set_chat
                >
                    "Chat"
                </button>
                <button
                    class="right-panel__tab"
                    class:right-panel__tab--active=move || active_tab() == RightTab::Ai
                    on:click=set_ai
                >
                    "AI"
                </button>
                <button
                    class="right-panel__tab"
                    class:right-panel__tab--active=move || active_tab() == RightTab::Boards
                    on:click=set_boards
                >
                    "Boards"
                </button>
            </div>

            <Show when=expanded>
                <div class="right-panel__content">
                    {move || match active_tab() {
                        RightTab::Chat => view! { <ChatPanel/> }.into_any(),
                        RightTab::Ai => view! { <AiPanel/> }.into_any(),
                        RightTab::Boards => view! { <MissionControl/> }.into_any(),
                    }}
                </div>
            </Show>
        </div>
    }
}
