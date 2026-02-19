//! Collapsible right panel with icon rail for boards, records, chat, and AI.
//!
//! ARCHITECTURE
//! ============
//! Right-side tools are organized as tabbed subviews so chat/AI/records can
//! share one rail without overlapping canvas layout logic.

use leptos::prelude::*;

use crate::components::ai_panel::AiPanel;
use crate::components::chat_panel::ChatPanel;
use crate::components::mission_control::MissionControl;
use crate::components::rewind_shelf::RewindShelf;
use crate::state::ui::{RightTab, UiState};

/// Collapsible right sidebar with icon rail and expandable content panel.
#[component]
pub fn RightPanel() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let expanded = move || ui.get().right_panel_expanded;
    let active_tab = move || ui.get().right_tab;

    let toggle_tab = move |tab: RightTab| {
        ui.update(|u| {
            if u.right_panel_expanded && u.right_tab == tab {
                u.right_panel_expanded = false;
            } else {
                u.right_panel_expanded = true;
                u.right_tab = tab;
            }
        });
    };

    let toggle_expand = move |_| {
        ui.update(|u| u.right_panel_expanded = !u.right_panel_expanded);
    };

    view! {
        <div class="right-panel">
            <div class="right-panel__rail">
                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Boards
                    on:click=move |_| toggle_tab(RightTab::Boards)
                    title="Boards"
                    attr:data-tooltip="Boards"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <rect x="3" y="3" width="6" height="6" />
                        <rect x="11" y="3" width="6" height="6" />
                        <rect x="3" y="11" width="6" height="6" />
                        <rect x="11" y="11" width="6" height="6" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Records
                    on:click=move |_| toggle_tab(RightTab::Records)
                    title="Field Records"
                    attr:data-tooltip="Field Records"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <ellipse cx="10" cy="5" rx="6" ry="2.5" />
                        <path d="M4 5 V10" />
                        <path d="M16 5 V10" />
                        <ellipse cx="10" cy="10" rx="6" ry="2.5" />
                        <path d="M4 10 V15" />
                        <path d="M16 10 V15" />
                        <ellipse cx="10" cy="15" rx="6" ry="2.5" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Chat
                    on:click=move |_| toggle_tab(RightTab::Chat)
                    title="Chat"
                    attr:data-tooltip="Chat"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <rect x="3" y="3" width="14" height="10" />
                        <path d="M7 13 L7 17 L11 13" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Ai
                    on:click=move |_| toggle_tab(RightTab::Ai)
                    title="Field Notes"
                    attr:data-tooltip="Field Notes"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <path d="M10 2 L12 7 L18 7 L13 10 L15 16 L10 12 L5 16 L7 10 L2 7 L8 7 Z" />
                    </svg>
                </button>

                <div class="right-panel__rail-spacer"></div>

                <button class="right-panel__toggle ui-tooltip ui-tooltip--left" on:click=toggle_expand title="Toggle panel" attr:data-tooltip="Toggle panel">
                    {move || if expanded() { "▶" } else { "◀" }}
                </button>
            </div>

            <Show when=expanded>
                <div class="right-panel__panel">
                    <div class="right-panel__header">
                        <span class="right-panel__title">
                            {move || match active_tab() {
                                RightTab::Chat => "Chat",
                                RightTab::Ai => "Field Notes",
                                RightTab::Boards => "Boards",
                                RightTab::Records => "Field Records",
                            }}
                        </span>
                        <button class="right-panel__close" on:click=move |_| ui.update(|u| u.right_panel_expanded = false)>
                            "✕"
                        </button>
                    </div>

                    <div class="right-panel__content">
                        {move || match active_tab() {
                            RightTab::Chat => view! { <ChatPanel/> }.into_any(),
                            RightTab::Ai => view! { <AiPanel/> }.into_any(),
                            RightTab::Boards => view! { <MissionControl/> }.into_any(),
                            RightTab::Records => view! { <RewindShelf/> }.into_any(),
                        }}
                    </div>
                </div>
            </Show>
        </div>
    }
}
