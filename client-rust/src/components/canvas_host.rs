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
    let last_presence_sent_ms = RwSignal::new(0.0_f64);
    #[cfg(feature = "hydrate")]
    let last_presence_sent = RwSignal::new(None::<(f64, f64, f64)>);
    #[cfg(feature = "hydrate")]
    let preview_cursor = RwSignal::new(None::<CanvasPoint>);
    let active_youtube = RwSignal::new(None::<(String, String)>);
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
            send_cursor_presence_if_needed(
                &instance,
                _board,
                _auth,
                _sender,
                last_presence_sent_ms,
                last_presence_sent,
                None,
                true,
            );
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
            for (id, obj) in &state.objects {
                let source = state.drag_objects.get(id).unwrap_or(obj);
                if let Some(mapped) = to_canvas_object(source, board_id.as_deref()) {
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
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                }
                sync_canvas_view_state(engine, _canvas_view, None);
                let _ = engine.render();
            }
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_follow = canvas_ref.clone();
        Effect::new(move || {
            let follow_client = _board.get().follow_client_id.clone();
            let jump_client = _board.get().jump_to_client_id.clone();
            let target_client = jump_client.or(follow_client);
            let Some(target_client) = target_client else {
                return;
            };
            let target_view = _board.get().presence.get(&target_client).cloned();
            let Some(target) = target_view else {
                return;
            };
            let Some(center) = target.camera_center else {
                return;
            };
            let Some(zoom) = target.camera_zoom else {
                return;
            };
            if let Some(engine) = engine.borrow_mut().as_mut() {
                sync_viewport(engine, &canvas_ref_follow);
                set_camera_view(engine, center.x, center.y, zoom);
                sync_canvas_view_state(engine, _canvas_view, None);
                let _ = engine.render();
                if _board.get_untracked().jump_to_client_id.as_deref() == Some(target_client.as_str()) {
                    _board.update(|b| b.jump_to_client_id = None);
                }
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
                let point = pointer_point(&ev);
                if let Some((kind, width, height, props)) = placement_shape(_ui.get().active_tool) {
                    if let Some(engine) = engine.borrow().as_ref() {
                        place_shape_at_cursor(point, kind, width, height, props, engine, _board, _sender);
                        _ui.update(|u| u.active_tool = ToolType::Select);
                        preview_cursor.set(None);
                        let _ = engine.render();
                    }
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_down(point, button, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    open_inspector_on_double_click(engine, &ev, _ui);
                    update_youtube_overlay_from_click(engine, point, &ev, active_youtube);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        Some(point),
                        false,
                    );
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
                let point = pointer_point(&ev);
                if placement_shape(_ui.get().active_tool).is_some() {
                    preview_cursor.set(Some(point));
                    if let Some(engine) = engine.borrow().as_ref() {
                        send_cursor_presence_if_needed(
                            engine,
                            _board,
                            _auth,
                            _sender,
                            last_presence_sent_ms,
                            last_presence_sent,
                            Some(point),
                            false,
                        );
                        sync_canvas_view_state(engine, _canvas_view, Some(point));
                    }
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_move(point, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        Some(point),
                        false,
                    );
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
                    let active_transform = active_transform_object_id(engine);
                    sync_viewport(engine, &canvas_ref);
                    let point = pointer_point(&ev);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_up(point, button, modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_selection_from_engine(engine, _board);
                    sync_canvas_view_state(engine, _canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        Some(point),
                        false,
                    );
                    last_drag_sent_ms.set(0.0);
                    send_object_drag_end(active_transform, _board, _sender);
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
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
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
                preview_cursor.set(None);
                if let Some(engine) = engine.borrow().as_ref() {
                    sync_canvas_view_state(engine, _canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                }
                send_cursor_clear(_board, _sender);
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
                let key = ev.key();
                if key == "Escape" && placement_shape(_ui.get().active_tool).is_some() {
                    ev.prevent_default();
                    _ui.update(|u| u.active_tool = ToolType::Select);
                    preview_cursor.set(None);
                    return;
                }
                if key == "Escape" {
                    active_youtube.set(None);
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    let active_transform = active_transform_object_id(engine);
                    sync_viewport(engine, &canvas_ref);
                    if should_prevent_default_key(&key) {
                        ev.prevent_default();
                    }
                    let key_for_engine = key.clone();
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_key_down(CanvasKey(key_for_engine), modifiers);
                    process_actions(actions, engine, _board, _sender);
                    sync_selection_from_engine(engine, _board);
                    sync_canvas_view_state(engine, _canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        _board,
                        _auth,
                        _sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    if key == "Escape" {
                        send_object_drag_end(active_transform, _board, _sender);
                    }
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
                    Some((p.client_id.clone(), p.name.clone(), p.color.clone(), screen_x, screen_y))
                })
                .collect::<Vec<_>>();
        }
        #[cfg(not(feature = "hydrate"))]
        {
            Vec::<(String, String, String, f64, f64)>::new()
        }
    };

    let preview_ghost = move || {
        #[cfg(feature = "hydrate")]
        {
            let Some((width, height, color)) = placement_preview(_ui.get().active_tool) else {
                return None::<(String, String)>;
            };
            let point = preview_cursor
                .get()
                .unwrap_or_else(|| CanvasPoint::new(40.0 + (width * 0.5), 40.0 + (height * 0.5)));
            let style = format!(
                "left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; background: {};",
                point.x - (width * 0.5),
                point.y - (height * 0.5),
                width,
                height,
                color
            );
            let class_name = if _ui.get().active_tool == ToolType::Sticky {
                "canvas-placement-ghost canvas-placement-ghost--sticky".to_owned()
            } else {
                "canvas-placement-ghost".to_owned()
            };
            Some((class_name, style))
        }
        #[cfg(not(feature = "hydrate"))]
        {
            None::<(String, String)>
        }
    };

    let youtube_overlay_open = move || active_youtube.get().is_some();

    let youtube_overlay_style = move || {
        #[cfg(feature = "hydrate")]
        {
            let Some((object_id, _video_id)) = active_youtube.get() else {
                return String::new();
            };
            let view = _canvas_view.get();
            let Some(obj) = _board.get().objects.get(&object_id).cloned() else {
                return String::new();
            };
            let left = (obj.x * view.zoom) + view.pan_x;
            let top = (obj.y * view.zoom) + view.pan_y;
            let width = obj.width.unwrap_or(320.0) * view.zoom;
            let height = obj.height.unwrap_or(220.0) * view.zoom;
            format!(
                "left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; transform: rotate({:.2}deg); transform-origin: center center;",
                left.max(0.0),
                top.max(0.0),
                width.max(80.0),
                height.max(60.0),
                obj.rotation
            )
        }
        #[cfg(not(feature = "hydrate"))]
        {
            String::new()
        }
    };

    let youtube_overlay_src = move || {
        #[cfg(feature = "hydrate")]
        {
            let Some((_object_id, video_id)) = active_youtube.get() else {
                return String::new();
            };
            return format!("https://www.youtube.com/embed/{}?autoplay=1&rel=0&modestbranding=1", video_id);
        }
        #[cfg(not(feature = "hydrate"))]
        {
            String::new()
        }
    };

    let youtube_overlay_frame_style = move || {
        #[cfg(feature = "hydrate")]
        {
            let Some((object_id, _video_id)) = active_youtube.get() else {
                return String::new();
            };
            let view = _canvas_view.get();
            let Some(obj) = _board.get().objects.get(&object_id).cloned() else {
                return String::new();
            };
            let width = obj.width.unwrap_or(320.0) * view.zoom;
            let height = obj.height.unwrap_or(220.0) * view.zoom;
            let (screen_x, screen_y, screen_w, screen_h) = youtube_screen_local_geometry(width, height);
            return format!(
                "left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px;",
                screen_x.max(0.0),
                screen_y.max(0.0),
                screen_w.max(32.0),
                screen_h.max(24.0)
            );
        }
        #[cfg(not(feature = "hydrate"))]
        {
            String::new()
        }
    };

    let camera_telemetry = move || {
        let view = _canvas_view.get();
        format!(
            "({}, {}) · {}%",
            view.camera_center_world.x.round() as i64,
            view.camera_center_world.y.round() as i64,
            (view.zoom * 100.0).round() as i64
        )
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
            {move || {
                preview_ghost().map(|(class_name, style)| {
                    view! { <div class=class_name style=style></div> }
                })
            }}
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
            <div class="canvas-camera-telemetry">{camera_telemetry}</div>
            <Show when=youtube_overlay_open>
                <div class="canvas-video-overlay" style=youtube_overlay_style>
                    <button class="canvas-video-overlay__close" on:click=move |_| active_youtube.set(None)>
                        "✕"
                    </button>
                    <iframe
                        class="canvas-video-overlay__frame"
                        style=youtube_overlay_frame_style
                        src=youtube_overlay_src
                        allow="autoplay; encrypted-media; picture-in-picture"
                        allowfullscreen=true
                        referrerpolicy="strict-origin-when-cross-origin"
                    ></iframe>
                </div>
            </Show>
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
fn set_camera_view(engine: &mut Engine, center_x: f64, center_y: f64, zoom: f64) {
    let clamped_zoom = zoom.clamp(0.1, 10.0);
    engine.core.camera.zoom = clamped_zoom;
    engine.core.camera.pan_x = (engine.core.viewport_width * 0.5) - (center_x * clamped_zoom);
    engine.core.camera.pan_y = (engine.core.viewport_height * 0.5) - (center_y * clamped_zoom);
}

#[cfg(feature = "hydrate")]
fn send_cursor_presence_if_needed(
    engine: &Engine,
    board: RwSignal<BoardState>,
    auth: RwSignal<AuthState>,
    sender: RwSignal<FrameSender>,
    last_sent_ms: RwSignal<f64>,
    last_sent_view: RwSignal<Option<(f64, f64, f64)>>,
    cursor_screen: Option<CanvasPoint>,
    force: bool,
) {
    let state = board.get_untracked();
    if state.connection_status != crate::state::board::ConnectionStatus::Connected {
        return;
    }
    if state.self_client_id.is_none() {
        return;
    }
    let Some(user) = auth.get_untracked().user else {
        return;
    };
    let has_cursor_point = cursor_screen.is_some();

    let now = now_ms();
    if !force && !has_cursor_point && now - last_sent_ms.get_untracked() < 120.0 {
        return;
    }
    let Some(board_id) = state.board_id else {
        return;
    };

    let camera = engine.camera();
    let center_screen = CanvasPoint::new(engine.core.viewport_width * 0.5, engine.core.viewport_height * 0.5);
    let center_world = camera.screen_to_world(center_screen);
    let center_x = center_world.x;
    let center_y = center_world.y;
    let zoom = camera.zoom;
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p));

    if !force
        && !has_cursor_point
        && let Some((last_x, last_y, last_zoom)) = last_sent_view.get_untracked()
    {
        let dx = center_x - last_x;
        let dy = center_y - last_y;
        let center_dist = (dx * dx + dy * dy).sqrt();
        let zoom_delta = (zoom - last_zoom).abs();
        if center_dist < 0.75 && zoom_delta < 0.003 {
            return;
        }
    }

    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "cursor:moved".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "x": cursor_world.as_ref().map(|p| p.x),
            "y": cursor_world.as_ref().map(|p| p.y),
            "name": user.name,
            "color": user.color,
            "camera_center_x": center_x,
            "camera_center_y": center_y,
            "camera_zoom": zoom,
        }),
    };
    if sender.get_untracked().send(&frame) {
        last_sent_ms.set(now);
        last_sent_view.set(Some((center_x, center_y, zoom)));
    }
}

#[cfg(feature = "hydrate")]
fn map_tool(tool: ToolType) -> CanvasTool {
    match tool {
        ToolType::Select => CanvasTool::Select,
        ToolType::Sticky | ToolType::Rectangle | ToolType::Frame | ToolType::Youtube => CanvasTool::Select,
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
fn open_inspector_on_double_click(engine: &Engine, ev: &leptos::ev::PointerEvent, ui: RwSignal<UiState>) {
    if ev.detail() < 2 {
        return;
    }
    if engine.selection().is_none() {
        return;
    }
    ui.update(|u| {
        u.left_panel_expanded = true;
    });
}

#[cfg(feature = "hydrate")]
fn update_youtube_overlay_from_click(
    engine: &Engine,
    point_screen: CanvasPoint,
    ev: &leptos::ev::PointerEvent,
    active_youtube: RwSignal<Option<(String, String)>>,
) {
    if ev.button() != 0 {
        return;
    }
    let world = engine.camera().screen_to_world(point_screen);
    let Some(selected_id) = youtube_object_at_point(engine, world) else {
        return;
    };
    let Some(obj) = engine.object(&selected_id) else {
        return;
    };
    if obj.kind != CanvasKind::Youtube {
        return;
    }
    if !youtube_play_button_hit(obj, world) && !youtube_screen_hit(obj, world) {
        return;
    }
    let Some(video_id) = youtube_video_id_from_props(&obj.props) else {
        return;
    };
    active_youtube.set(Some((selected_id.to_string(), video_id)));
}

#[cfg(feature = "hydrate")]
fn youtube_object_at_point(engine: &Engine, world: CanvasPoint) -> Option<uuid::Uuid> {
    engine
        .core
        .doc
        .sorted_objects()
        .into_iter()
        .rev()
        .find(|obj| {
            obj.kind == CanvasKind::Youtube
                && canvas::hit::point_in_rect(world, obj.x, obj.y, obj.width, obj.height, obj.rotation)
        })
        .map(|obj| obj.id)
}

#[cfg(feature = "hydrate")]
fn youtube_play_button_hit(obj: &CanvasObject, world: CanvasPoint) -> bool {
    let local = canvas::hit::world_to_local(world, obj.x, obj.y, obj.width, obj.height, obj.rotation);
    let (cx, cy, r) = youtube_play_button_local_geometry(obj.width, obj.height);
    let dx = local.x - cx;
    let dy = local.y - cy;
    (dx * dx) + (dy * dy) <= (r * r)
}

#[cfg(feature = "hydrate")]
fn youtube_screen_hit(obj: &CanvasObject, world: CanvasPoint) -> bool {
    let local = canvas::hit::world_to_local(world, obj.x, obj.y, obj.width, obj.height, obj.rotation);
    let (_, screen_y, screen_w, screen_h) = youtube_screen_local_geometry(obj.width, obj.height);
    let screen_x = (obj.width - screen_w) * 0.5;
    local.x >= screen_x && local.x <= screen_x + screen_w && local.y >= screen_y && local.y <= screen_y + screen_h
}

#[cfg(feature = "hydrate")]
fn youtube_play_button_local_geometry(width: f64, height: f64) -> (f64, f64, f64) {
    let (_screen_x, screen_y, _screen_w, screen_h) = youtube_screen_local_geometry(width, height);
    let cy = screen_y + (screen_h * 0.5);
    let cx = width * 0.5;
    let r = width.min(height) * 0.12;
    (cx, cy, r)
}

#[cfg(feature = "hydrate")]
fn youtube_screen_local_geometry(width: f64, height: f64) -> (f64, f64, f64, f64) {
    let bezel_pad_x = width * 0.08;
    let bezel_pad_y = height * 0.14;
    let bezel_w = width - (bezel_pad_x * 2.0);
    let bezel_h = height - (bezel_pad_y * 2.0) - (height * 0.10);
    let screen_pad = width.min(height) * 0.04;
    let screen_w = bezel_w - (screen_pad * 2.0);
    let screen_h = bezel_h - (screen_pad * 2.0);
    let screen_x = (width - screen_w) * 0.5;
    let screen_y = bezel_pad_y + screen_pad;
    (screen_x, screen_y, screen_w, screen_h)
}

#[cfg(feature = "hydrate")]
fn youtube_video_id_from_props(props: &serde_json::Value) -> Option<String> {
    let raw = props.get("video_id").and_then(|v| v.as_str())?;
    parse_youtube_video_id(raw)
}

#[cfg(feature = "hydrate")]
fn parse_youtube_video_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed.contains("youtube.com") && !trimmed.contains("youtu.be/") {
        return sanitize_youtube_id(trimmed);
    }
    if let Some(idx) = trimmed.find("v=") {
        let v = &trimmed[idx + 2..];
        let end = v.find(['&', '#', '?']).unwrap_or(v.len());
        return sanitize_youtube_id(&v[..end]);
    }
    if let Some(idx) = trimmed.find("youtu.be/") {
        let v = &trimmed[idx + "youtu.be/".len()..];
        let end = v.find(['&', '#', '?', '/']).unwrap_or(v.len());
        return sanitize_youtube_id(&v[..end]);
    }
    if let Some(idx) = trimmed.find("/embed/") {
        let v = &trimmed[idx + "/embed/".len()..];
        let end = v.find(['&', '#', '?', '/']).unwrap_or(v.len());
        return sanitize_youtube_id(&v[..end]);
    }
    None
}

#[cfg(feature = "hydrate")]
fn sanitize_youtube_id(id: &str) -> Option<String> {
    let cleaned = id.trim();
    if cleaned.len() < 8 || cleaned.len() > 15 {
        return None;
    }
    if cleaned
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        Some(cleaned.to_owned())
    } else {
        None
    }
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
    let camera_center_screen = CanvasPoint::new(engine.core.viewport_width * 0.5, engine.core.viewport_height * 0.5);
    let camera_center_world = camera.screen_to_world(camera_center_screen);
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p));

    canvas_view.update(|v| {
        v.cursor_world = cursor_world.map(|p| WirePoint { x: p.x, y: p.y });
        v.camera_center_world = WirePoint { x: camera_center_world.x, y: camera_center_world.y };
        v.zoom = camera.zoom;
        v.pan_x = camera.pan_x;
        v.pan_y = camera.pan_y;
    });
}

#[cfg(feature = "hydrate")]
fn active_transform_object_id(engine: &Engine) -> Option<String> {
    match engine.core.input.clone() {
        CanvasInputState::DraggingObject { id, .. }
        | CanvasInputState::ResizingObject { id, .. }
        | CanvasInputState::RotatingObject { id, .. }
        | CanvasInputState::DraggingEdgeEndpoint { id, .. } => Some(id.to_string()),
        _ => None,
    }
}

#[cfg(feature = "hydrate")]
fn send_cursor_clear(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "cursor:clear".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({}),
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
fn send_object_drag_end(id: Option<String>, board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let Some(id) = id else {
        return;
    };
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "object:drag:end".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({ "id": id }),
    };
    let _ = sender.get_untracked().send(&frame);
}

#[cfg(feature = "hydrate")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn remote_cursor_style(x: f64, y: f64, color: &str) -> String {
    format!("transform: translate({x:.2}px, {y:.2}px); --cursor-color: {color};")
}

#[cfg(feature = "hydrate")]
fn placement_shape(tool: ToolType) -> Option<(&'static str, f64, f64, serde_json::Value)> {
    match tool {
        ToolType::Sticky => Some((
            "sticky_note",
            120.0,
            120.0,
            serde_json::json!({
                "title": "New note",
                "text": "",
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 1
            }),
        )),
        ToolType::Rectangle => Some((
            "rectangle",
            160.0,
            100.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 1
            }),
        )),
        ToolType::Frame => Some((
            "frame",
            520.0,
            320.0,
            serde_json::json!({
                "title": "Frame",
                "color": "#9AA3AD",
                "backgroundColor": "rgba(154,163,173,0.08)",
                "borderColor": "#1F1A17",
                "borderWidth": 2,
                "stroke": "#1F1A17",
                "stroke_width": 2
            }),
        )),
        ToolType::Ellipse => Some((
            "ellipse",
            120.0,
            120.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 1
            }),
        )),
        ToolType::Youtube => Some((
            "youtube_embed",
            320.0,
            220.0,
            serde_json::json!({
                "video_id": "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=RDdQw4w9WgXcQ&start_radio=1",
                "title": "YouTube",
                "stroke": "#1F1A17",
                "stroke_width": 2
            }),
        )),
        ToolType::Line => Some((
            "line",
            180.0,
            0.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 2
            }),
        )),
        ToolType::Connector => Some((
            "arrow",
            180.0,
            0.0,
            serde_json::json!({
                "color": "#D94B4B",
                "backgroundColor": "#D94B4B",
                "borderColor": "#D94B4B",
                "borderWidth": 2
            }),
        )),
        _ => None,
    }
}

#[cfg(feature = "hydrate")]
fn placement_preview(tool: ToolType) -> Option<(f64, f64, &'static str)> {
    match tool {
        ToolType::Sticky => Some((120.0, 120.0, "rgba(217, 75, 75, 0.5)")),
        ToolType::Rectangle => Some((160.0, 100.0, "rgba(217, 75, 75, 0.5)")),
        ToolType::Frame => Some((520.0, 320.0, "rgba(154, 163, 173, 0.20)")),
        ToolType::Ellipse => Some((120.0, 120.0, "rgba(217, 75, 75, 0.5)")),
        ToolType::Youtube => Some((320.0, 220.0, "rgba(217, 75, 75, 0.45)")),
        ToolType::Line => Some((180.0, 2.0, "rgba(217, 75, 75, 0.65)")),
        ToolType::Connector => Some((180.0, 2.0, "rgba(217, 75, 75, 0.65)")),
        _ => None,
    }
}

#[cfg(feature = "hydrate")]
fn place_shape_at_cursor(
    point_screen: CanvasPoint,
    kind: &str,
    width: f64,
    height: f64,
    props: serde_json::Value,
    engine: &Engine,
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
) {
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };

    let world = engine.camera().screen_to_world(point_screen);
    let x = world.x - (width * 0.5);
    let y = world.y - (height * 0.5);
    let id = uuid::Uuid::new_v4().to_string();
    let props = materialize_shape_props(kind, x, y, width, height, props);

    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "object:create".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "id": id,
            "kind": kind,
            "x": x,
            "y": y,
            "width": width,
            "height": height,
            "rotation": 0,
            "props": props,
        }),
    };
    let _ = sender.get_untracked().send(&frame);
}

#[cfg(feature = "hydrate")]
fn materialize_shape_props(
    kind: &str,
    x: f64,
    y: f64,
    width: f64,
    _height: f64,
    props: serde_json::Value,
) -> serde_json::Value {
    if kind != "line" && kind != "arrow" {
        return props;
    }
    let mut map = match props {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    map.insert("a".to_owned(), serde_json::json!({ "x": x, "y": y }));
    map.insert("b".to_owned(), serde_json::json!({ "x": x + width, "y": y }));
    serde_json::Value::Object(map)
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
        "frame" => CanvasKind::Frame,
        "ellipse" => CanvasKind::Ellipse,
        "diamond" => CanvasKind::Diamond,
        "star" => CanvasKind::Star,
        "youtube_embed" | "youtube" => CanvasKind::Youtube,
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
        CanvasKind::Frame => "frame",
        CanvasKind::Ellipse => "ellipse",
        CanvasKind::Diamond => "diamond",
        CanvasKind::Star => "star",
        CanvasKind::Youtube => "youtube_embed",
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
