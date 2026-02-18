//! Collapsible left panel containing the tool rail and inspector panel.

use leptos::prelude::*;

use crate::components::inspector_panel::InspectorPanel;
use crate::components::tool_rail::ToolRail;
use crate::components::tool_strip::ToolStrip;
use crate::state::ui::{LeftTab, UiState};

/// Collapsible left sidebar with tab switching between Tools and Inspector.
#[component]
pub fn LeftPanel() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let expanded = move || ui.get().left_panel_expanded;
    let active_tab = move || ui.get().left_tab;

    let toggle_expand = move |_| {
        ui.update(|u| u.left_panel_expanded = !u.left_panel_expanded);
    };

    let set_tools = move |_| {
        ui.update(|u| u.left_tab = LeftTab::Tools);
    };

    let set_inspector = move |_| {
        ui.update(|u| u.left_tab = LeftTab::Inspector);
    };

    view! {
        <div class="left-panel" class:left-panel--collapsed=move || !expanded()>
            <div class="left-panel__tabs">
                <button
                    class="left-panel__tab"
                    class:left-panel__tab--active=move || active_tab() == LeftTab::Tools
                    on:click=set_tools
                >
                    "Tools"
                </button>
                <button
                    class="left-panel__tab"
                    class:left-panel__tab--active=move || active_tab() == LeftTab::Inspector
                    on:click=set_inspector
                >
                    "Inspector"
                </button>
                <button class="left-panel__toggle" on:click=toggle_expand>
                    {move || if expanded() { "\u{25C0}" } else { "\u{25B6}" }}
                </button>
            </div>

            <Show when=expanded>
                <div class="left-panel__content">
                    {move || match active_tab() {
                        LeftTab::Tools => {
                            view! {
                                <ToolRail/>
                                <ToolStrip/>
                            }
                                .into_any()
                        }
                        LeftTab::Inspector => {
                            view! { <InspectorPanel/> }.into_any()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}
