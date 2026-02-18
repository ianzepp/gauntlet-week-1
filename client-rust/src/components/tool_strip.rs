//! Quick-create buttons with shape and color presets.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::Frame;
use crate::state::board::BoardState;
use crate::state::ui::UiState;

/// Preset shape colors.
const PRESET_COLORS: &[&str] = &["#3498db", "#2ecc71", "#e74c3c", "#f39c12", "#9b59b6", "#1abc9c"];

/// Quick-create strip for the active tool.
///
/// Shows color presets and an "Add" button that creates an object at a
/// default position via the `object:create` frame.
#[component]
pub fn ToolStrip() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let selected_color = RwSignal::new(PRESET_COLORS[0].to_owned());

    let on_add = move |_| {
        let tool = ui.get().active_tool;
        let kind = format!("{tool:?}").to_lowercase();
        let color = selected_color.get();
        let board_id = board.get().board_id.clone();

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0.0,
            board_id,
            from: None,
            syscall: "object:create".to_owned(),
            status: crate::net::types::FrameStatus::Request,
            data: serde_json::json!({
                "kind": kind,
                "x": 100.0,
                "y": 100.0,
                "width": 120.0,
                "height": 80.0,
                "props": { "fill": color }
            }),
        };
        sender.get().send(&frame);
    };

    view! {
        <div class="tool-strip">
            <div class="tool-strip__colors">
                {PRESET_COLORS
                    .iter()
                    .map(|&c| {
                        let color = c.to_owned();
                        let color_val = color.clone();
                        let is_selected = {
                            let color = color.clone();
                            move || selected_color.get() == color
                        };
                        view! {
                            <button
                                class="tool-strip__color"
                                class:tool-strip__color--selected=is_selected
                                style:background=color_val
                                on:click=move |_| selected_color.set(color.clone())
                            ></button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
            <button class="btn btn--primary tool-strip__add" on:click=on_add>
                "Add"
            </button>
        </div>
    }
}
