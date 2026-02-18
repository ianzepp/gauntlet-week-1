//! Bridge component between Leptos state and the imperative `canvas::Engine`.

use leptos::prelude::*;

use crate::state::board::BoardState;
use crate::state::ui::UiState;
#[cfg(feature = "hydrate")]
use crate::state::ui::ToolType;

#[cfg(feature = "hydrate")]
use std::cell::RefCell;
#[cfg(feature = "hydrate")]
use std::rc::Rc;

#[cfg(feature = "hydrate")]
use canvas::doc::{BoardObject as CanvasObject, ObjectKind as CanvasKind};
#[cfg(feature = "hydrate")]
use canvas::engine::Engine;
#[cfg(feature = "hydrate")]
use canvas::input::Tool as CanvasTool;

/// Canvas host component.
///
/// On hydration, this mounts `canvas::engine::Engine`, synchronizes board
/// objects from websocket state, and renders on updates.
#[component]
pub fn CanvasHost() -> impl IntoView {
    let _board = expect_context::<RwSignal<BoardState>>();
    let _ui = expect_context::<RwSignal<UiState>>();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    #[cfg(feature = "hydrate")]
    let engine = Rc::new(RefCell::new(None::<Engine>));

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_mount = canvas_ref.clone();
        Effect::new(move || {
            let Some(canvas) = canvas_ref_mount.get() else {
                return;
            };
            if engine.borrow().is_some() {
                return;
            }

            let mut instance = Engine::new(canvas);
            sync_viewport(&mut instance, &canvas_ref_mount);
            let _ = instance.render();
            *engine.borrow_mut() = Some(instance);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_sync = canvas_ref.clone();
        Effect::new(move || {
            let mut snapshot = Vec::new();
            let state = _board.get();
            let board_id = state.board_id.clone();
            for obj in state.objects.values() {
                if let Some(mapped) = to_canvas_object(obj, board_id.as_deref()) {
                    snapshot.push(mapped);
                }
            }

            let tool = map_tool(_ui.get().active_tool);
            if let Some(engine) = engine.borrow_mut().as_mut() {
                engine.load_snapshot(snapshot);
                engine.set_tool(tool);
                sync_viewport(engine, &canvas_ref_sync);
                let _ = engine.render();
            }
        });
    }

    view! {
        <canvas class="canvas-host" node_ref=canvas_ref tabindex="0">
            "Your browser does not support canvas."
        </canvas>
    }
}

#[cfg(feature = "hydrate")]
fn sync_viewport(engine: &mut Engine, canvas_ref: &NodeRef<leptos::html::Canvas>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(canvas) = canvas_ref.get() else {
        return;
    };
    let width = f64::from(canvas.client_width()).max(1.0);
    let height = f64::from(canvas.client_height()).max(1.0);
    let dpr = window.device_pixel_ratio().max(1.0);
    engine.set_viewport(width, height, dpr);
}

#[cfg(feature = "hydrate")]
fn map_tool(tool: ToolType) -> CanvasTool {
    match tool {
        ToolType::Select => CanvasTool::Select,
        ToolType::Sticky | ToolType::Rectangle => CanvasTool::Rect,
        ToolType::Ellipse => CanvasTool::Ellipse,
        ToolType::Line | ToolType::Connector => CanvasTool::Line,
        ToolType::Text | ToolType::Draw | ToolType::Eraser => CanvasTool::Select,
    }
}

#[cfg(feature = "hydrate")]
fn to_canvas_object(obj: &crate::net::types::BoardObject, active_board_id: Option<&str>) -> Option<CanvasObject> {
    let id = uuid::Uuid::parse_str(&obj.id).ok()?;
    let board_id = active_board_id
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .or_else(|| uuid::Uuid::parse_str(&obj.board_id).ok())
        .unwrap_or(uuid::Uuid::nil());
    let created_by = obj
        .created_by
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    let kind = match obj.kind.as_str() {
        "rectangle" | "rect" | "sticky_note" => CanvasKind::Rect,
        "ellipse" => CanvasKind::Ellipse,
        "diamond" => CanvasKind::Diamond,
        "star" => CanvasKind::Star,
        "line" => CanvasKind::Line,
        "arrow" => CanvasKind::Arrow,
        _ => CanvasKind::Rect,
    };

    let width = obj.width.unwrap_or(120.0).max(1.0);
    let height = obj.height.unwrap_or(80.0).max(1.0);
    let mut props = obj.props.clone();
    if let Some(map) = props.as_object_mut() {
        if let Some(v) = map.get("backgroundColor").and_then(|v| v.as_str()).map(ToOwned::to_owned) {
            map.entry("fill".to_owned())
                .or_insert_with(|| serde_json::Value::String(v));
        }
        if let Some(v) = map.get("borderColor").and_then(|v| v.as_str()).map(ToOwned::to_owned) {
            map.entry("stroke".to_owned())
                .or_insert_with(|| serde_json::Value::String(v));
        }
        if let Some(v) = map.get("borderWidth").and_then(|v| v.as_i64()) {
            map.entry("stroke_width".to_owned())
                .or_insert_with(|| serde_json::json!(v));
        }
    }

    Some(CanvasObject {
        id,
        board_id,
        kind,
        x: obj.x,
        y: obj.y,
        width,
        height,
        rotation: obj.rotation,
        z_index: i64::from(obj.z_index),
        props,
        created_by,
        version: obj.version,
    })
}
