//! Vertical strip of tool buttons for selecting the active drawing tool.

use leptos::prelude::*;

use crate::state::ui::{ToolType, UiState};

/// Tool definition for the rail.
struct ToolDef {
    tool: ToolType,
    label: &'static str,
    icon: &'static str,
}

const TOOLS: &[ToolDef] = &[
    ToolDef { tool: ToolType::Select, label: "Select", icon: "\u{25B3}" },
    ToolDef { tool: ToolType::Rect, label: "Rectangle", icon: "\u{25A1}" },
    ToolDef { tool: ToolType::Ellipse, label: "Ellipse", icon: "\u{25CB}" },
    ToolDef { tool: ToolType::Diamond, label: "Diamond", icon: "\u{25C7}" },
    ToolDef { tool: ToolType::Star, label: "Star", icon: "\u{2606}" },
    ToolDef { tool: ToolType::Line, label: "Line", icon: "\u{2014}" },
    ToolDef { tool: ToolType::Arrow, label: "Arrow", icon: "\u{2192}" },
];

/// Vertical strip of tool selection buttons.
///
/// Highlights the active tool from `UiState.active_tool` and updates it on click.
#[component]
pub fn ToolRail() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let buttons = TOOLS
        .iter()
        .map(|td| {
            let tool = td.tool;
            let label = td.label;
            let icon = td.icon;

            let is_active = move || ui.get().active_tool == tool;
            let on_click = move |_| {
                ui.update(|u| u.active_tool = tool);
            };

            view! {
                <button
                    class="tool-rail__btn"
                    class:tool-rail__btn--active=is_active
                    title=label
                    on:click=on_click
                >
                    {icon}
                </button>
            }
        })
        .collect::<Vec<_>>();

    view! { <div class="tool-rail">{buttons}</div> }
}
