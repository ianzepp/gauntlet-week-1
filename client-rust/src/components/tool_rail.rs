//! Vertical strip of tool buttons for selecting the active drawing tool.

use leptos::prelude::*;

use crate::state::ui::{ToolType, UiState};

#[derive(Clone, Copy)]
struct ToolDef {
    tool: ToolType,
    label: &'static str,
    disabled: bool,
}

const PRIMARY_TOOLS: &[ToolDef] = &[ToolDef { tool: ToolType::Select, label: "Select", disabled: false }];

const SHAPE_TOOLS: &[ToolDef] = &[
    ToolDef { tool: ToolType::Sticky, label: "Note", disabled: false },
    ToolDef { tool: ToolType::Rectangle, label: "Rectangle", disabled: false },
    ToolDef { tool: ToolType::Ellipse, label: "Circle", disabled: false },
    ToolDef { tool: ToolType::Line, label: "Line", disabled: false },
    ToolDef { tool: ToolType::Connector, label: "Arrow", disabled: false },
    ToolDef { tool: ToolType::Text, label: "Text", disabled: true },
];

const DRAW_TOOLS: &[ToolDef] = &[
    ToolDef { tool: ToolType::Draw, label: "Draw", disabled: true },
    ToolDef { tool: ToolType::Eraser, label: "Eraser", disabled: true },
];

/// Vertical strip of tool selection buttons with a tool-strip flyout.
#[component]
pub fn ToolRail() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let render_group = move |tools: &'static [ToolDef]| {
        tools
            .iter()
            .map(|td| {
                let td = *td;
                let title = if td.disabled {
                    format!("{} (coming soon)", td.label)
                } else {
                    td.label.to_owned()
                };

                let is_active = move || ui.get().active_tool == td.tool;

                let on_click = move |_ev: leptos::ev::MouseEvent| {
                    if td.disabled {
                        return;
                    }
                    ui.update(|u| u.active_tool = td.tool);
                };

                view! {
                    <button
                        class="tool-rail__btn"
                        class:tool-rail__btn--active=is_active
                        class:tool-rail__btn--disabled=move || td.disabled
                        title=title
                        disabled=td.disabled
                        on:click=on_click
                    >
                        {render_icon(td.tool)}
                    </button>
                }
            })
            .collect::<Vec<_>>()
    };

    let toggle_expand = move |_| {
        ui.update(|u| u.left_panel_expanded = !u.left_panel_expanded);
    };

    let expanded = move || ui.get().left_panel_expanded;

    view! {
        <div class="tool-rail">
            {render_group(PRIMARY_TOOLS)}
            <div class="tool-rail__separator"></div>
            {render_group(SHAPE_TOOLS)}
            <div class="tool-rail__separator"></div>
            {render_group(DRAW_TOOLS)}

            <div class="tool-rail__spacer"></div>

            <button class="tool-rail__toggle" on:click=toggle_expand title="Toggle inspector">
                {move || if expanded() { "◀" } else { "▶" }}
            </button>
        </div>
    }
}

fn render_icon(tool: ToolType) -> impl IntoView {
    match tool {
        ToolType::Select => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M4 2 L4 16 L8 12 L12 18 L14 17 L10 11 L15 11 Z" />
            </svg>
        }
        .into_any(),
        ToolType::Sticky => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="2" y="2" width="16" height="16" />
                <line x1="5" y1="7" x2="15" y2="7" />
                <line x1="5" y1="11" x2="12" y2="11" />
            </svg>
        }
        .into_any(),
        ToolType::Rectangle => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="2" y="4" width="16" height="12" />
            </svg>
        }
        .into_any(),
        ToolType::Ellipse => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <ellipse cx="10" cy="10" rx="8" ry="6" />
            </svg>
        }
        .into_any(),
        ToolType::Line => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <line x1="3" y1="17" x2="17" y2="3" />
            </svg>
        }
        .into_any(),
        ToolType::Connector => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <line x1="3" y1="17" x2="17" y2="3" />
                <polyline points="10,3 17,3 17,10" />
            </svg>
        }
        .into_any(),
        ToolType::Text => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <line x1="4" y1="4" x2="16" y2="4" />
                <line x1="10" y1="4" x2="10" y2="17" />
                <line x1="7" y1="17" x2="13" y2="17" />
            </svg>
        }
        .into_any(),
        ToolType::Draw => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M3 17 L14 6 L16 4 L17 3 L14 6" />
                <path d="M14 6 L16 8" />
                <line x1="3" y1="17" x2="5" y2="15" />
            </svg>
        }
        .into_any(),
        ToolType::Eraser => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M8 16 L3 11 L11 3 L18 10 L13 16 Z" />
                <line x1="3" y1="16" x2="13" y2="16" />
            </svg>
        }
        .into_any(),
    }
}
