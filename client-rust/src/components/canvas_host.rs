//! Bridge component between Leptos state and the imperative `canvas::Engine`.

use leptos::prelude::*;

use crate::app::FrameSender;
#[cfg(feature = "hydrate")]
use crate::net::types::{BoardObject, Frame, FrameStatus, Point as WirePoint};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::canvas_view::CanvasViewState;
#[cfg(feature = "hydrate")]
use crate::state::ui::ToolType;
use crate::state::ui::UiState;

#[cfg(feature = "hydrate")]
use std::cell::RefCell;
#[cfg(feature = "hydrate")]
use std::rc::Rc;

#[cfg(feature = "hydrate")]
use canvas::camera::Point as CanvasPoint;
#[cfg(feature = "hydrate")]
use canvas::doc::{BoardObject as CanvasObject, ObjectKind as CanvasKind};
#[cfg(feature = "hydrate")]
use canvas::engine::{Action, Engine};
#[cfg(feature = "hydrate")]
use canvas::input::{
    Button as CanvasButton, InputState as CanvasInputState, Key as CanvasKey, Modifiers as CanvasModifiers,
    Tool as CanvasTool, WheelDelta,
};

/// Canvas host component.
///
/// On hydration, this mounts `canvas::engine::Engine`, synchronizes board
/// objects from websocket state, and renders on updates.
#[component]
pub fn CanvasHost() -> impl IntoView {
    let _auth = expect_context::<RwSignal<AuthState>>();
    let _board = expect_context::<RwSignal<BoardState>>();
    let _canvas_view = expect_context::<RwSignal<CanvasViewState>>();
    let _sender = expect_context::<RwSignal<FrameSender>>();
    let _ui = expect_context::<RwSignal<UiState>>();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    #[cfg(feature = "hydrate")]
    let last_centered_board = RwSignal::new(None::<String>);
    #[cfg(feature = "hydrate")]
    let last_drag_sent_ms = RwSignal::new(0.0_f64);
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
            center_world_origin(&mut instance);
            sync_canvas_view_state(&instance, _canvas_view, None);
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
                if last_centered_board.get_untracked() != board_id {
                    center_world_origin(engine);
                    last_centered_board.set(board_id.clone());
                }
                sync_canvas_view_state(engine, _canvas_view, None);
                let _ = engine.render();
            }
        });
    }

    let on_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                ev.prevent_default();
                if let Some(canvas) = canvas_ref.get() {
                    let _ = canvas.focus();
                    let _ = canvas.set_pointer_capture(ev.pointer_id());
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let point = pointer_point(&ev);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_down(point, button, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let point = pointer_point(&ev);
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_move(point, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    send_cursor_moved(engine, point, _auth, _board, _sender);
                    send_object_drag_if_needed(engine, _board, _sender, last_drag_sent_ms);
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                if let Some(canvas) = canvas_ref.get() {
                    let _ = canvas.release_pointer_capture(ev.pointer_id());
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let point = pointer_point(&ev);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_up(point, button, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_selection_from_engine(engine, _board);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    last_drag_sent_ms.set(0.0);
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_wheel = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::WheelEvent| {
                ev.prevent_default();
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let point = wheel_point(&ev);
                    let delta = WheelDelta { dx: ev.delta_x(), dy: ev.delta_y() };
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_wheel(point, delta, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::WheelEvent| {}
        }
    };

    let on_pointer_leave = {
        #[cfg(feature = "hydrate")]
        {
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::PointerEvent| {
                if let Some(engine) = engine.borrow().as_ref() {
                    sync_canvas_view_state(engine, _canvas_view, None);
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_key_down = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::KeyboardEvent| {
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let key = ev.key();
                    if should_prevent_default_key(&key) {
                        ev.prevent_default();
                    }
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_key_down(CanvasKey(key), modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_selection_from_engine(engine, _board);
                    sync_canvas_view_state(engine, _canvas_view, None);
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::KeyboardEvent| {}
        }
    };

    let remote_cursors = move || {
        #[cfg(feature = "hydrate")]
        {
            let view = _canvas_view.get();
            let pan_x = view.pan_x;
            let pan_y = view.pan_y;
            let zoom = view.zoom;
            return _board
                .get()
                .presence
                .values()
                .filter_map(|p| {
                    let cursor = p.cursor.as_ref()?;
                    let screen_x = cursor.x * zoom + pan_x;
                    let screen_y = cursor.y * zoom + pan_y;
                    Some((p.user_id.clone(), p.name.clone(), p.color.clone(), screen_x, screen_y))
                })
                .collect::<Vec<_>>();
        }
        #[cfg(not(feature = "hydrate"))]
        {
            Vec::<(String, String, String, f64, f64)>::new()
        }
    };

    view! {
        <>
            <canvas
                class="canvas-host"
                node_ref=canvas_ref
                tabindex="0"
                on:pointerdown=on_pointer_down
                on:pointermove=on_pointer_move
                on:pointerup=on_pointer_up
                on:pointerleave=on_pointer_leave
                on:wheel=on_wheel
                on:keydown=on_key_down
            >
                "Your browser does not support canvas."
            </canvas>
            <div class="canvas-cursors">
                {move || {
                    remote_cursors()
                        .into_iter()
                        .map(|(_id, name, color, x, y)| {
                            let style = remote_cursor_style(x, y, &color);
                            let title = name.clone();
                            view! {
                                <div class="canvas-cursor" style=style title=title>
                                    <span class="canvas-cursor__name">{name}</span>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </>
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
fn center_world_origin(engine: &mut Engine) {
    engine.core.camera.pan_x = engine.core.viewport_width * 0.5;
    engine.core.camera.pan_y = engine.core.viewport_height * 0.5;
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
fn map_button(button: i16) -> CanvasButton {
    match button {
        1 => CanvasButton::Middle,
        2 => CanvasButton::Secondary,
        _ => CanvasButton::Primary,
    }
}

#[cfg(feature = "hydrate")]
fn map_modifiers(shift: bool, ctrl: bool, alt: bool, meta: bool) -> CanvasModifiers {
    CanvasModifiers { shift, ctrl, alt, meta }
}

#[cfg(feature = "hydrate")]
fn should_prevent_default_key(key: &str) -> bool {
    matches!(key, "Delete" | "Backspace" | "Escape" | "Enter")
}

#[cfg(feature = "hydrate")]
fn pointer_point(ev: &leptos::ev::PointerEvent) -> CanvasPoint {
    CanvasPoint::new(f64::from(ev.offset_x()), f64::from(ev.offset_y()))
}

#[cfg(feature = "hydrate")]
fn wheel_point(ev: &leptos::ev::WheelEvent) -> CanvasPoint {
    CanvasPoint::new(f64::from(ev.offset_x()), f64::from(ev.offset_y()))
}

#[cfg(feature = "hydrate")]
fn sync_selection_from_engine(engine: &Engine, board: RwSignal<BoardState>) {
    let selected = engine.selection().map(|id| id.to_string());
    board.update(|b| {
        if b.selection.len() <= 1 && b.selection.iter().next().cloned() == selected {
            return;
        }
        b.selection.clear();
        if let Some(id) = selected
            && b.objects.contains_key(&id)
        {
            b.selection.insert(id);
        }
    });
}

#[cfg(feature = "hydrate")]
fn sync_canvas_view_state(engine: &Engine, canvas_view: RwSignal<CanvasViewState>, cursor_screen: Option<CanvasPoint>) {
    let camera = engine.camera();
    let viewport_center_screen = CanvasPoint::new(engine.core.viewport_width * 0.5, engine.core.viewport_height * 0.5);
    let viewport_center_world = camera.screen_to_world(viewport_center_screen);
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p));

    canvas_view.update(|v| {
        v.cursor_world = cursor_world.map(|p| WirePoint { x: p.x, y: p.y });
        v.viewport_center_world = WirePoint { x: viewport_center_world.x, y: viewport_center_world.y };
        v.zoom = camera.zoom;
        v.pan_x = camera.pan_x;
        v.pan_y = camera.pan_y;
    });
}

#[cfg(feature = "hydrate")]
fn send_cursor_moved(
    engine: &Engine,
    point_screen: CanvasPoint,
    auth: RwSignal<AuthState>,
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
) {
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let Some(user) = auth.get_untracked().user else {
        return;
    };
    let world = engine.camera().screen_to_world(point_screen);
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "cursor:moved".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "x": world.x,
            "y": world.y,
            "name": user.name,
            "color": user.color
        }),
    };
    let _ = sender.get_untracked().send(&frame);
}

#[cfg(feature = "hydrate")]
fn send_object_drag_if_needed(
    engine: &Engine,
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    last_sent_ms: RwSignal<f64>,
) {
    let now = now_ms();
    if now - last_sent_ms.get_untracked() < 33.0 {
        return;
    }

    let object_id = match engine.core.input.clone() {
        CanvasInputState::DraggingObject { id, .. }
        | CanvasInputState::ResizingObject { id, .. }
        | CanvasInputState::RotatingObject { id, .. }
        | CanvasInputState::DraggingEdgeEndpoint { id, .. } => id,
        _ => return,
    };

    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let Some(obj) = engine.object(&object_id) else {
        return;
    };

    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "object:drag".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "id": obj.id.to_string(),
            "x": obj.x,
            "y": obj.y,
            "width": obj.width,
            "height": obj.height,
            "rotation": obj.rotation,
            "z_index": obj.z_index,
            "props": obj.props,
        }),
    };
    if sender.get_untracked().send(&frame) {
        last_sent_ms.set(now);
    }
}

#[cfg(feature = "hydrate")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn remote_cursor_style(x: f64, y: f64, color: &str) -> String {
    format!("transform: translate({x:.2}px, {y:.2}px); --cursor-color: {color};")
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
        if let Some(v) = map
            .get("backgroundColor")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
        {
            map.entry("fill".to_owned())
                .or_insert_with(|| serde_json::Value::String(v));
        }
        if let Some(v) = map
            .get("borderColor")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
        {
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

#[cfg(feature = "hydrate")]
fn process_actions(
    actions: Vec<Action>,
    engine: &mut Engine,
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
) {
    for action in actions {
        match action {
            Action::ObjectCreated(obj) => {
                let Some(board_id) = board.get_untracked().board_id else {
                    continue;
                };
                if let Some(local) = to_wire_object(&obj, &board_id) {
                    board.update(|b| {
                        b.objects.insert(local.id.clone(), local.clone());
                        b.selection.clear();
                        b.selection.insert(local.id.clone());
                    });
                }

                let frame = Frame {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: None,
                    ts: 0,
                    board_id: Some(board_id),
                    from: None,
                    syscall: "object:create".to_owned(),
                    status: FrameStatus::Request,
                    data: serde_json::json!({
                        "id": obj.id.to_string(),
                        "kind": canvas_kind_to_wire(obj.kind),
                        "x": obj.x,
                        "y": obj.y,
                        "width": obj.width,
                        "height": obj.height,
                        "rotation": obj.rotation,
                        "props": obj.props,
                    }),
                };
                let _ = sender.get_untracked().send(&frame);
            }
            Action::ObjectUpdated { id, fields } => {
                let Some(board_id) = board.get_untracked().board_id else {
                    continue;
                };
                if let Some(obj) = engine.object(&id)
                    && let Some(local) = to_wire_object(obj, &board_id)
                {
                    board.update(|b| {
                        b.objects.insert(local.id.clone(), local);
                    });
                }

                let mut data = serde_json::Map::new();
                data.insert("id".to_owned(), serde_json::json!(id.to_string()));
                if let Some(x) = fields.x {
                    data.insert("x".to_owned(), serde_json::json!(x));
                }
                if let Some(y) = fields.y {
                    data.insert("y".to_owned(), serde_json::json!(y));
                }
                if let Some(width) = fields.width {
                    data.insert("width".to_owned(), serde_json::json!(width));
                }
                if let Some(height) = fields.height {
                    data.insert("height".to_owned(), serde_json::json!(height));
                }
                if let Some(rotation) = fields.rotation {
                    data.insert("rotation".to_owned(), serde_json::json!(rotation));
                }
                if let Some(z) = fields.z_index {
                    data.insert("z_index".to_owned(), serde_json::json!(z));
                }
                if let Some(props) = fields.props {
                    data.insert("props".to_owned(), props);
                }
                if let Some(obj) = engine.object(&id) {
                    data.insert("version".to_owned(), serde_json::json!(obj.version));
                } else if let Some(version) = fields.version {
                    data.insert("version".to_owned(), serde_json::json!(version));
                }

                let frame = Frame {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: None,
                    ts: 0,
                    board_id: Some(board_id),
                    from: None,
                    syscall: "object:update".to_owned(),
                    status: FrameStatus::Request,
                    data: serde_json::Value::Object(data),
                };
                let _ = sender.get_untracked().send(&frame);
            }
            Action::ObjectDeleted { id } => {
                let Some(board_id) = board.get_untracked().board_id else {
                    continue;
                };
                let id_string = id.to_string();
                board.update(|b| {
                    b.objects.remove(&id_string);
                    b.selection.remove(&id_string);
                });

                let frame = Frame {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: None,
                    ts: 0,
                    board_id: Some(board_id),
                    from: None,
                    syscall: "object:delete".to_owned(),
                    status: FrameStatus::Request,
                    data: serde_json::json!({ "id": id_string }),
                };
                let _ = sender.get_untracked().send(&frame);
            }
            Action::None | Action::RenderNeeded | Action::EditTextRequested { .. } | Action::SetCursor(_) => {}
        }
    }
}

#[cfg(feature = "hydrate")]
fn canvas_kind_to_wire(kind: CanvasKind) -> &'static str {
    match kind {
        CanvasKind::Rect => "rectangle",
        CanvasKind::Ellipse => "ellipse",
        CanvasKind::Diamond => "diamond",
        CanvasKind::Star => "star",
        CanvasKind::Line => "line",
        CanvasKind::Arrow => "arrow",
    }
}

#[cfg(feature = "hydrate")]
fn to_wire_object(obj: &CanvasObject, board_id: &str) -> Option<BoardObject> {
    let z_index = i32::try_from(obj.z_index).ok()?;
    Some(BoardObject {
        id: obj.id.to_string(),
        board_id: board_id.to_owned(),
        kind: canvas_kind_to_wire(obj.kind).to_owned(),
        x: obj.x,
        y: obj.y,
        width: Some(obj.width),
        height: Some(obj.height),
        rotation: obj.rotation,
        z_index,
        props: obj.props.clone(),
        created_by: obj.created_by.map(|u| u.to_string()),
        version: obj.version,
    })
}
