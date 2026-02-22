//! Vertical strip of tool buttons for selecting the active drawing tool.
//!
//! DESIGN
//! ======
//! Keeps active-tool switching centralized so other components can treat tool
//! choice as state, not direct DOM coupling.

use leptos::prelude::*;

use crate::state::board::BoardState;
use crate::state::ui::{ToolType, UiState};

const INSPECTOR_ENABLED: bool = false;

#[derive(Clone, Copy)]
struct ToolDef {
    tool: ToolType,
    label: &'static str,
    disabled: bool,
}

const PRIMARY_TOOLS: &[ToolDef] = &[
    ToolDef { tool: ToolType::Hand, label: "Hand", disabled: false },
    ToolDef { tool: ToolType::Select, label: "Select", disabled: false },
];

const SHAPE_TOOLS: &[ToolDef] = &[
    ToolDef { tool: ToolType::Sticky, label: "Note", disabled: false },
    ToolDef { tool: ToolType::Rectangle, label: "Rectangle", disabled: false },
    ToolDef { tool: ToolType::Frame, label: "Frame", disabled: false },
    ToolDef { tool: ToolType::Ellipse, label: "Circle", disabled: false },
    ToolDef { tool: ToolType::Line, label: "Line", disabled: false },
    ToolDef { tool: ToolType::Connector, label: "Arrow", disabled: false },
    ToolDef { tool: ToolType::Text, label: "Text", disabled: false },
];

const DRAW_TOOLS: &[ToolDef] = &[
    ToolDef { tool: ToolType::Draw, label: "Draw", disabled: true },
    ToolDef { tool: ToolType::Eraser, label: "Eraser", disabled: true },
];

/// Vertical strip of tool selection buttons with a tool-strip flyout.
#[component]
pub fn ToolRail() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();
    let board = expect_context::<RwSignal<BoardState>>();

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
                        class="tool-rail__btn ui-tooltip"
                        class:tool-rail__btn--active=is_active
                        class:tool-rail__btn--disabled=move || td.disabled
                        title=title.clone()
                        attr:data-tooltip=title
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
        if !INSPECTOR_ENABLED {
            return;
        }
        ui.update(|u| u.left_panel_expanded = !u.left_panel_expanded);
    };

    let expanded = move || INSPECTOR_ENABLED && ui.get().left_panel_expanded;
    let on_home_click = move |_ev: leptos::ev::MouseEvent| {
        board.update(|b| {
            b.follow_client_id = None;
            b.jump_to_client_id = None;
        });
        ui.update(|u| {
            u.home_viewport_seq = u.home_viewport_seq.saturating_add(1);
        });
    };

    view! {
        <div class="tool-rail">
            <button class="tool-rail__btn ui-tooltip" title="Home" attr:data-tooltip="Home" on:click=on_home_click>
                {render_home_icon()}
            </button>
            <div class="tool-rail__separator"></div>
            {render_group(PRIMARY_TOOLS)}
            <div class="tool-rail__separator"></div>
            {render_group(SHAPE_TOOLS)}
            <div class="tool-rail__separator"></div>
            {render_group(DRAW_TOOLS)}

            <div class="tool-rail__spacer"></div>

            <Show when=move || INSPECTOR_ENABLED>
                <button class="tool-rail__toggle ui-tooltip" on:click=toggle_expand title="Toggle inspector" attr:data-tooltip="Toggle inspector">
                    {move || if expanded() { "◀" } else { "▶" }}
                </button>
            </Show>
        </div>
    }
}

fn render_home_icon() -> impl IntoView {
    view! {
        <svg viewBox="0 0 20 20" aria-hidden="true">
            <path d="M3 9.5 L10 3 L17 9.5" />
            <path d="M5.5 8.5 V16 H14.5 V8.5" />
            <path d="M8.5 16 V11.5 H11.5 V16" />
        </svg>
    }
}

fn render_icon(tool: ToolType) -> impl IntoView {
    match tool {
        ToolType::Hand => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M6 10.5 V5.5 C6 4.8 6.5 4.2 7.2 4.2 C7.9 4.2 8.4 4.8 8.4 5.5 V9.5" />
                <path d="M8.4 9.5 V4.2 C8.4 3.5 8.9 3 9.6 3 C10.3 3 10.8 3.5 10.8 4.2 V9.5" />
                <path d="M10.8 9.5 V4.8 C10.8 4.1 11.3 3.6 12 3.6 C12.7 3.6 13.2 4.1 13.2 4.8 V10.2" />
                <path d="M13.2 8.6 C13.2 8 13.7 7.5 14.3 7.5 C15 7.5 15.5 8 15.5 8.6 V12.3 C15.5 15.1 13.2 17.4 10.4 17.4 H9.3 C6.8 17.4 4.8 15.6 4.5 13.2 L4 10.2 C3.9 9.5 4.4 8.9 5 8.8 C5.6 8.7 6.2 9.1 6.3 9.8 L6.6 11.3" />
            </svg>
        }
        .into_any(),
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
        ToolType::Frame => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="2" y="3" width="16" height="14" />
                <line x1="2" y1="7" x2="18" y2="7" />
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
