//! Collapsible left panel containing a tool rail and inspector panel.
//!
//! ARCHITECTURE
//! ============
//! Keeps creation tools and selection inspector co-located so editing controls
//! remain spatially consistent with canvas interactions.

use leptos::prelude::*;

use crate::components::inspector_panel::InspectorPanel;
use crate::components::tool_rail::ToolRail;
use crate::state::ui::UiState;

/// Left sidebar with a fixed tool rail and expandable inspector panel.
#[component]
pub fn LeftPanel() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let expanded = move || ui.get().left_panel_expanded;
    let toggle_expand = move |_| {
        ui.update(|u| u.left_panel_expanded = !u.left_panel_expanded);
    };

    view! {
        <div class="left-panel">
            <Show when=expanded>
                <div class="left-panel__panel">
                    <div class="left-panel__header">
                        <span class="left-panel__title">"Inspector"</span>
                        <button class="left-panel__close" on:click=toggle_expand>
                            "✕"
                        </button>
                    </div>
                    <div class="left-panel__content">
                        <InspectorPanel/>
                    </div>
                </div>
            </Show>

            <ToolRail/>
        </div>
    }
}
