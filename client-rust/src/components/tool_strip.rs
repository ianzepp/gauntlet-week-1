//! Quick-create flyout for sticky notes and rectangles.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{BoardObject, Frame, FrameStatus};
use crate::state::board::BoardState;
use crate::state::ui::ToolType;

struct ShapePreset {
    label: &'static str,
    width: f64,
    height: f64,
}

const SHAPE_PRESETS: &[ShapePreset] = &[
    ShapePreset {
        label: "Square",
        width: 120.0,
        height: 120.0,
    },
    ShapePreset {
        label: "Tall",
        width: 100.0,
        height: 160.0,
    },
    ShapePreset {
        label: "Wide",
        width: 200.0,
        height: 100.0,
    },
];

const COLOR_PRESETS: &[(&str, &str)] = &[
    ("Red", "#D94B4B"),
    ("Blue", "#4B7DD9"),
    ("Green", "#4BAF6E"),
];

/// Tool strip flyout used by sticky and rectangle tools.
#[component]
pub fn ToolStrip(tool_type: ToolType, open_strip: RwSignal<Option<ToolType>>) -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let shape_index = RwSignal::new(0_usize);
    let color_index = RwSignal::new(0_usize);

    let on_add = move |_| {
        let shape = &SHAPE_PRESETS[shape_index.get()];
        let (_, color) = COLOR_PRESETS[color_index.get()];

        let id = uuid::Uuid::new_v4().to_string();
        let board_id = board.get().board_id.unwrap_or_default();
        let kind = match tool_type {
            ToolType::Sticky => "sticky_note",
            _ => "rectangle",
        };

        let props = match tool_type {
            ToolType::Sticky => serde_json::json!({
                "title": "New note",
                "text": "",
                "color": color,
                "backgroundColor": color,
                "borderColor": color,
                "borderWidth": 1
            }),
            _ => serde_json::json!({
                "color": color,
                "backgroundColor": color,
                "borderColor": color,
                "borderWidth": 1
            }),
        };

        let x = 400.0 - (shape.width / 2.0);
        let y = 300.0 - (shape.height / 2.0);

        let new_object = BoardObject {
            id: id.clone(),
            board_id: board_id.clone(),
            kind: kind.to_owned(),
            x,
            y,
            width: Some(shape.width),
            height: Some(shape.height),
            rotation: 0.0,
            z_index: board.get().objects.len() as i32,
            props: props.clone(),
            created_by: Some("local".to_owned()),
            version: 1,
        };

        board.update(|b| {
            b.objects.insert(id.clone(), new_object.clone());
            b.selection.clear();
            b.selection.insert(id.clone());
        });

        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0.0,
            board_id: Some(board_id),
            from: None,
            syscall: "object:create".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": id,
                "kind": kind,
                "x": x,
                "y": y,
                "width": shape.width,
                "height": shape.height,
                "rotation": 0,
                "props": props,
            }),
        };
        sender.get().send(&frame);

        open_strip.set(None);
    };

    view! {
        <div class="tool-strip">
            <div class="tool-strip__options">
                {SHAPE_PRESETS
                    .iter()
                    .enumerate()
                    .map(|(idx, preset)| {
                        let is_active = move || shape_index.get() == idx;
                        view! {
                            <button
                                class="tool-strip__option"
                                class:tool-strip__option--active=is_active
                                title=preset.label
                                on:click=move |_| shape_index.set(idx)
                            >
                                {shape_icon(idx)}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>

            <div class="tool-strip__divider"></div>

            <div class="tool-strip__options">
                {COLOR_PRESETS
                    .iter()
                    .enumerate()
                    .map(|(idx, (label, color))| {
                        let color = (*color).to_owned();
                        let is_active = move || color_index.get() == idx;
                        view! {
                            <button
                                class="tool-strip__swatch"
                                class:tool-strip__swatch--active=is_active
                                title=*label
                                on:click=move |_| color_index.set(idx)
                            >
                                <span class="tool-strip__swatch-color" style:background=color></span>
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>

            <div class="tool-strip__divider"></div>

            <button
                class="tool-strip__add"
                style:background=move || COLOR_PRESETS[color_index.get()].1
                on:click=on_add
            >
                "Add"
            </button>
        </div>
    }
}

fn shape_icon(idx: usize) -> impl IntoView {
    match idx {
        0 => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="3" y="3" width="14" height="14" />
            </svg>
        }
        .into_any(),
        1 => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="5" y="2" width="10" height="16" />
            </svg>
        }
        .into_any(),
        _ => view! {
            <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="2" y="5" width="16" height="10" />
            </svg>
        }
        .into_any(),
    }
}
