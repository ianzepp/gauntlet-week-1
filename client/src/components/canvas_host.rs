//! Bridge component between Leptos state and the imperative `canvas::Engine`.
//!
//! ARCHITECTURE
//! ============
//! The canvas crate owns render-time performance concerns while this host maps
//! websocket/state events into engine operations and publishes viewport telemetry.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::components::dial::{ColorDial, CompassDial, ZoomDial};
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

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionRotationDragState {
    start_pointer_angle_deg: f64,
    start_rotations: Vec<(String, f64)>,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionScaleSeed {
    id: String,
    board_id: String,
    version: i64,
    base_width: f64,
    base_height: f64,
    start_scale: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionScaleDragState {
    start_items: Vec<SelectionScaleSeed>,
    group_center_x: f64,
    group_center_y: f64,
    start_group_scale: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionColorSeed {
    id: String,
    board_id: String,
    version: i64,
    start_fill: String,
    start_base_fill: String,
    start_lightness_shift: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionColorDragState {
    start_items: Vec<SelectionColorSeed>,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionBorderSeed {
    id: String,
    board_id: String,
    version: i64,
    start_border_color: String,
    start_border_width: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
struct SelectionBorderDragState {
    start_items: Vec<SelectionBorderSeed>,
}

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
    let compass_ref = NodeRef::<leptos::html::Div>::new();
    let _compass_drag_active = RwSignal::new(false);
    let zoom_ref = NodeRef::<leptos::html::Div>::new();
    let _zoom_drag_active = RwSignal::new(false);
    let object_zoom_ref = NodeRef::<leptos::html::Div>::new();
    let _object_zoom_drag_active = RwSignal::new(false);
    let object_rotate_ref = NodeRef::<leptos::html::Div>::new();
    let _object_rotate_drag_active = RwSignal::new(false);
    let object_color_ref = NodeRef::<leptos::html::Div>::new();
    let _object_color_drag_active = RwSignal::new(false);
    let object_border_ref = NodeRef::<leptos::html::Div>::new();
    let _object_border_drag_active = RwSignal::new(false);
    #[cfg(feature = "hydrate")]
    let object_rotate_drag_state = RwSignal::new(None::<SelectionRotationDragState>);
    #[cfg(feature = "hydrate")]
    let object_zoom_drag_state = RwSignal::new(None::<SelectionScaleDragState>);
    #[cfg(feature = "hydrate")]
    let object_color_drag_state = RwSignal::new(None::<SelectionColorDragState>);
    #[cfg(feature = "hydrate")]
    let object_border_drag_state = RwSignal::new(None::<SelectionBorderDragState>);
    #[cfg(feature = "hydrate")]
    let last_centered_board = RwSignal::new(None::<String>);
    #[cfg(feature = "hydrate")]
    let last_drag_sent_ms = RwSignal::new(0.0_f64);
    #[cfg(feature = "hydrate")]
    let last_presence_sent_ms = RwSignal::new(0.0_f64);
    #[cfg(feature = "hydrate")]
    let last_presence_sent = RwSignal::new(None::<(f64, f64, f64, f64)>);
    #[cfg(feature = "hydrate")]
    let last_presence_bootstrap_key = RwSignal::new(None::<(String, String)>);
    #[cfg(feature = "hydrate")]
    let last_home_viewport_seq = RwSignal::new(0_u64);
    #[cfg(feature = "hydrate")]
    let last_zoom_override_seq = RwSignal::new(0_u64);
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
        let canvas_ref_bootstrap = canvas_ref.clone();
        Effect::new(move || {
            let state = _board.get();
            if state.connection_status != crate::state::board::ConnectionStatus::Connected {
                return;
            }
            let Some(board_id) = state.board_id.clone() else {
                return;
            };
            let Some(client_id) = state.self_client_id.clone() else {
                return;
            };
            let key = (board_id, client_id);
            if last_presence_bootstrap_key.get().as_ref() == Some(&key) {
                return;
            }
            if let Some(engine) = engine.borrow_mut().as_mut() {
                sync_viewport(engine, &canvas_ref_bootstrap);
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
                last_presence_bootstrap_key.set(Some(key));
            }
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
        let canvas_ref_home = canvas_ref.clone();
        Effect::new(move || {
            let seq = _ui.get().home_viewport_seq;
            if seq == last_home_viewport_seq.get_untracked() {
                return;
            }
            if let Some(engine) = engine.borrow_mut().as_mut() {
                sync_viewport(engine, &canvas_ref_home);
                center_world_origin(engine);
                sync_canvas_view_state(engine, _canvas_view, None);
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
                let _ = engine.render();
            }
            last_home_viewport_seq.set(seq);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_zoom = canvas_ref.clone();
        Effect::new(move || {
            let ui_state = _ui.get();
            let seq = ui_state.zoom_override_seq;
            if seq == 0 || seq == last_zoom_override_seq.get_untracked() {
                return;
            }
            let target_zoom = ui_state.zoom_override;
            if let Some(engine) = engine.borrow_mut().as_mut() {
                if let Some(zoom) = target_zoom {
                    sync_viewport(engine, &canvas_ref_zoom);
                    let center_screen = viewport_center_screen(engine);
                    let center_world = engine
                        .camera()
                        .screen_to_world(center_screen, center_screen);
                    let rotation = engine.view_rotation_deg();
                    set_camera_view(engine, center_world.x, center_world.y, zoom, rotation);
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
            last_zoom_override_seq.set(seq);
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
            let rotation = target.camera_rotation.unwrap_or(0.0);
            if let Some(engine) = engine.borrow_mut().as_mut() {
                sync_viewport(engine, &canvas_ref_follow);
                set_camera_view(engine, center.x, center.y, zoom, rotation);
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
                if _board.get().follow_client_id.is_some() {
                    return;
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

    let on_double_click = {
        #[cfg(feature = "hydrate")]
        {
            move |ev: leptos::ev::MouseEvent| {
                if ev.button() != 0 {
                    return;
                }
                if _board.get().selection.is_empty() {
                    return;
                }
                _ui.update(|u| {
                    u.object_text_dialog_seq = u.object_text_dialog_seq.saturating_add(1);
                });
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                let point = pointer_point(&ev);
                if _board.get().follow_client_id.is_some() {
                    if let Some(engine) = engine.borrow().as_ref() {
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
                    }
                    return;
                }
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
                if _board.get().follow_client_id.is_some() {
                    return;
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
                if _board.get().follow_client_id.is_some() {
                    return;
                }
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
                if _board.get().follow_client_id.is_some() {
                    if key == "Escape" {
                        active_youtube.set(None);
                    }
                    return;
                }
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

    let on_compass_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let compass_ref = compass_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                ev.prevent_default();
                ev.stop_propagation();
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                let Some(compass) = compass_ref.get() else {
                    return;
                };
                let _ = compass.set_pointer_capture(ev.pointer_id());

                _compass_drag_active.set(true);
                if let Some(angle) = compass_angle_from_pointer(&ev, &compass)
                    && let Some(engine) = engine.borrow_mut().as_mut()
                {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(apply_compass_drag_snapping(angle, ev.shift_key()));
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_compass_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let compass_ref = compass_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                if !_compass_drag_active.get_untracked() || _board.get().follow_client_id.is_some() {
                    return;
                }
                let Some(compass) = compass_ref.get() else {
                    return;
                };
                if let Some(angle) = compass_angle_from_pointer(&ev, &compass)
                    && let Some(engine) = engine.borrow_mut().as_mut()
                {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(apply_compass_drag_snapping(angle, ev.shift_key()));
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_compass_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::PointerEvent| {
                _compass_drag_active.set(false);
                if let Some(engine) = engine.borrow().as_ref() {
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
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_compass_snap_n = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::MouseEvent| {
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(0.0);
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_compass_snap_e = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::MouseEvent| {
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(90.0);
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_compass_snap_s = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::MouseEvent| {
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(180.0);
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_compass_snap_w = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::MouseEvent| {
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(270.0);
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_compass_center_click = move |_ev: leptos::ev::MouseEvent| {};

    let on_compass_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
    };

    let on_zoom_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let zoom_ref = zoom_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                ev.prevent_default();
                ev.stop_propagation();
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                let Some(zoom) = zoom_ref.get() else {
                    return;
                };
                let _ = zoom.set_pointer_capture(ev.pointer_id());
                _zoom_drag_active.set(true);
                if let Some(angle) = zoom_angle_from_pointer(&ev, &zoom)
                    && let Some(engine) = engine.borrow_mut().as_mut()
                {
                    sync_viewport(engine, &canvas_ref);
                    zoom_view_preserving_center(engine, zoom_from_dial_angle(angle));
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_zoom_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let zoom_ref = zoom_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                if !_zoom_drag_active.get_untracked() || _board.get().follow_client_id.is_some() {
                    return;
                }
                let Some(zoom) = zoom_ref.get() else {
                    return;
                };
                if let Some(angle) = zoom_angle_from_pointer(&ev, &zoom)
                    && let Some(engine) = engine.borrow_mut().as_mut()
                {
                    sync_viewport(engine, &canvas_ref);
                    zoom_view_preserving_center(engine, zoom_from_dial_angle(angle));
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_zoom_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::PointerEvent| {
                _zoom_drag_active.set(false);
                if let Some(engine) = engine.borrow().as_ref() {
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
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_zoom_reset = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |_ev: leptos::ev::MouseEvent| {
                if _board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    zoom_view_preserving_center(engine, 1.0);
                    sync_canvas_view_state(engine, _canvas_view, None);
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
                    let _ = engine.render();
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_zoom_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    };

    let on_object_color_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let object_color_ref = object_color_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if pointer_event_hits_control(&ev, ".canvas-color-dial__picker, .canvas-color-dial__readout") {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                if !has_selection(_board) {
                    return;
                }
                let Some(dial) = object_color_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_color_seed(_board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_color_drag_state.set(Some(drag_state));
                _object_color_drag_active.set(true);
                apply_selection_color_shift(_board, object_color_drag_state, color_shift_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_color_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let object_color_ref = object_color_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if !_object_color_drag_active.get_untracked() {
                    return;
                }
                let Some(dial) = object_color_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                apply_selection_color_shift(_board, object_color_drag_state, color_shift_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_color_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::PointerEvent| {
                _object_color_drag_active.set(false);
                commit_selection_color_updates(_board, _sender, object_color_drag_state);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_color_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    };

    let on_object_color_input = {
        #[cfg(feature = "hydrate")]
        {
            move |ev: leptos::ev::Event| {
                use wasm_bindgen::JsCast;

                let Some(input) = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                else {
                    return;
                };
                apply_group_base_color_target(_board, _sender, input.value());
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::Event| {}
        }
    };

    let on_object_border_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let object_border_ref = object_border_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if pointer_event_hits_control(&ev, ".canvas-color-dial__picker, .canvas-color-dial__readout") {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                if !has_selection(_board) {
                    return;
                }
                let Some(dial) = object_border_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_border_seed(_board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_border_drag_state.set(Some(drag_state));
                _object_border_drag_active.set(true);
                apply_selection_border_width(_board, object_border_drag_state, border_width_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_border_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let object_border_ref = object_border_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if !_object_border_drag_active.get_untracked() {
                    return;
                }
                let Some(dial) = object_border_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                apply_selection_border_width(_board, object_border_drag_state, border_width_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_border_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::PointerEvent| {
                _object_border_drag_active.set(false);
                commit_selection_border_updates(_board, _sender, object_border_drag_state);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_border_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    };

    let on_object_border_input = {
        #[cfg(feature = "hydrate")]
        {
            move |ev: leptos::ev::Event| {
                use wasm_bindgen::JsCast;

                let Some(input) = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                else {
                    return;
                };
                apply_group_border_color_target(_board, _sender, input.value());
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::Event| {}
        }
    };
    let on_object_color_reset = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_background_defaults_target(_board, _sender)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_border_reset = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_border_defaults_target(_board, _sender)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_object_zoom_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let object_zoom_ref = object_zoom_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if pointer_event_hits_control(&ev, ".canvas-zoom-wheel__marker, .canvas-zoom-wheel__readout") {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                if !has_selection(_board) {
                    return;
                }
                let Some(dial) = object_zoom_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_scale_seed(_board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_zoom_drag_state.set(Some(drag_state));
                _object_zoom_drag_active.set(true);
                apply_selection_scale_drag(_board, object_zoom_drag_state, zoom_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_zoom_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let object_zoom_ref = object_zoom_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if !_object_zoom_drag_active.get_untracked() {
                    return;
                }
                let Some(dial) = object_zoom_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                apply_selection_scale_drag(_board, object_zoom_drag_state, zoom_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_zoom_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::PointerEvent| {
                _object_zoom_drag_active.set(false);
                commit_selection_scale_updates(_board, _sender, object_zoom_drag_state);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_zoom_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    };

    let object_color_base = move || selection_representative_base_color_hex(_board);
    let object_color_shift = move || selection_representative_lightness_shift(_board);
    let object_color_knob_style = move || {
        let angle = dial_angle_from_color_shift(object_color_shift());
        format!("transform: rotate({angle:.2}deg);")
    };
    let object_border_color = move || selection_representative_border_color_hex(_board);
    let object_border_width = move || selection_representative_border_width(_board);
    let object_border_knob_style = move || {
        let angle = dial_angle_from_border_width(object_border_width());
        format!("transform: rotate({angle:.2}deg);")
    };

    let object_zoom_scale = move || selection_representative_scale_factor(_board);
    let object_zoom_knob_style = move || {
        let angle = dial_angle_from_zoom(object_zoom_scale());
        format!("transform: rotate({angle:.2}deg);")
    };

    let on_object_rotate_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let object_rotate_ref = object_rotate_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                ev.prevent_default();
                ev.stop_propagation();
                let Some(dial) = object_rotate_ref.get() else {
                    return;
                };
                let Some(angle) = compass_angle_from_pointer(&ev, &dial) else {
                    return;
                };

                let start_rotations = selected_object_rotations(_board);
                if start_rotations.is_empty() {
                    return;
                }
                let _ = dial.set_pointer_capture(ev.pointer_id());

                object_rotate_drag_state.set(Some(SelectionRotationDragState {
                    start_pointer_angle_deg: angle,
                    start_rotations,
                }));
                _object_rotate_drag_active.set(true);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_rotate_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let object_rotate_ref = object_rotate_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if !_object_rotate_drag_active.get_untracked() {
                    return;
                }
                let Some(dial) = object_rotate_ref.get() else {
                    return;
                };
                let Some(angle) = compass_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                apply_selection_rotation_drag(_board, object_rotate_drag_state, angle, ev.shift_key());
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_rotate_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::PointerEvent| {
                _object_rotate_drag_active.set(false);
                commit_selection_rotation_updates(_board, _sender, object_rotate_drag_state);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_object_rotate_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
    };

    let on_object_rotate_snap_n = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_rotation_target(_board, _sender, 0.0)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_snap_e = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_rotation_target(_board, _sender, 90.0)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_snap_s = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_rotation_target(_board, _sender, 180.0)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_snap_w = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_rotation_target(_board, _sender, 270.0)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_center_click = move |_ev: leptos::ev::MouseEvent| {};

    let object_rotation_angle_deg = move || selection_representative_rotation_deg(_board);
    let object_rotation_knob_style = move || {
        let angle = object_rotation_angle_deg();
        format!("transform: rotate({angle:.2}deg);")
    };
    let has_selected_objects = move || !_board.get().selection.is_empty();

    let compass_angle_deg = move || normalize_degrees_360(_canvas_view.get().view_rotation_deg);
    let compass_knob_style = move || {
        let angle = compass_angle_deg();
        format!("transform: rotate({angle:.2}deg);")
    };
    let zoom_percent = move || _canvas_view.get().zoom * 100.0;
    let zoom_knob_style = move || {
        #[cfg(feature = "hydrate")]
        {
            let angle = dial_angle_from_zoom(_canvas_view.get().zoom);
            return format!("transform: rotate({angle:.2}deg);");
        }
        #[cfg(not(feature = "hydrate"))]
        {
            "transform: rotate(0deg);".to_owned()
        }
    };

    let canvas_world_overlay_style = move || {
        let view = _canvas_view.get();
        let cx = view.viewport_width * 0.5;
        let cy = view.viewport_height * 0.5;
        format!(
            "transform: translate({cx:.2}px, {cy:.2}px) rotate({:.2}deg) translate({:.2}px, {:.2}px); transform-origin: 0 0;",
            view.view_rotation_deg, -cx, -cy
        )
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
                on:dblclick=on_double_click
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
            <div class="canvas-world-overlay" style=canvas_world_overlay_style>
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
            </div>
            <ZoomDial
                class="canvas-object-zoom"
                disabled_class="canvas-object-zoom--disabled"
                title="Drag to scale selected object(s); top is neutral"
                readout_title="Click to reset selected object scale to 100%"
                knob_class="canvas-object-zoom__knob"
                node_ref=object_zoom_ref
                disabled=Signal::derive(move || !has_selected_objects())
                readout=Signal::derive(move || format!("{:.0}%", object_zoom_scale() * 100.0))
                knob_style=Signal::derive(object_zoom_knob_style)
                on_pointer_down=on_object_zoom_pointer_down
                on_pointer_move=on_object_zoom_pointer_move
                on_pointer_up=on_object_zoom_pointer_up
                on_readout_pointer_down=on_object_zoom_readout_pointer_down
                on_readout_click=move |_ev| apply_group_scale_target(_board, _sender, 1.0)
                on_readout_dblclick=move |_ev| apply_group_scale_target(_board, _sender, 1.0)
            />
            <CompassDial
                class="canvas-object-rotate"
                disabled_class="canvas-object-rotate--disabled"
                title="Drag to rotate selected object(s); hold Shift to snap by 15deg"
                readout_title="Selected object/group rotation"
                knob_class="canvas-object-rotate__knob"
                node_ref=object_rotate_ref
                disabled=Signal::derive(move || !has_selected_objects())
                readout=Signal::derive(move || format!("{:.0}deg", object_rotation_angle_deg()))
                knob_style=Signal::derive(object_rotation_knob_style)
                on_pointer_down=on_object_rotate_pointer_down
                on_pointer_move=on_object_rotate_pointer_move
                on_pointer_up=on_object_rotate_pointer_up
                on_snap_n=on_object_rotate_snap_n
                on_snap_e=on_object_rotate_snap_e
                on_snap_s=on_object_rotate_snap_s
                on_snap_w=on_object_rotate_snap_w
                on_readout_pointer_down=on_object_rotate_readout_pointer_down
                on_readout_click=on_object_rotate_center_click
                on_readout_dblclick=on_object_rotate_center_click
            />
            <CompassDial
                class="canvas-compass"
                disabled_class=""
                title="Drag to rotate view; hold Shift to snap by 15deg"
                readout_title="Board rotation"
                knob_class="canvas-compass__knob"
                node_ref=compass_ref
                disabled=Signal::derive(|| false)
                readout=Signal::derive(move || format!("{:.0}deg", compass_angle_deg()))
                knob_style=Signal::derive(compass_knob_style)
                on_pointer_down=on_compass_pointer_down
                on_pointer_move=on_compass_pointer_move
                on_pointer_up=on_compass_pointer_up
                on_snap_n=on_compass_snap_n
                on_snap_e=on_compass_snap_e
                on_snap_s=on_compass_snap_s
                on_snap_w=on_compass_snap_w
                on_readout_pointer_down=on_compass_readout_pointer_down
                on_readout_click=on_compass_center_click
                on_readout_dblclick=on_compass_center_click
            />
            <ZoomDial
                class="canvas-zoom-wheel"
                disabled_class=""
                title="Drag around dial to zoom"
                readout_title="Click to reset zoom to 100%"
                knob_class="canvas-zoom-wheel__knob"
                node_ref=zoom_ref
                disabled=Signal::derive(|| false)
                readout=Signal::derive(move || format!("{:.0}%", zoom_percent()))
                knob_style=Signal::derive(zoom_knob_style)
                on_pointer_down=on_zoom_pointer_down
                on_pointer_move=on_zoom_pointer_move
                on_pointer_up=on_zoom_pointer_up
                on_readout_pointer_down=on_zoom_readout_pointer_down
                on_readout_click=on_zoom_reset.clone()
                on_readout_dblclick=on_zoom_reset
            />
            <ColorDial
                class="canvas-object-color"
                disabled_class="canvas-object-color--disabled"
                title="Drag to shift selected color lightness; center picks base color"
                swatch_title="Selected base color"
                reset_title="Reset background to defaults"
                center_label=Signal::derive(move || format!("{:+.0}", object_color_shift() * 100.0))
                knob_class="canvas-object-color__knob"
                node_ref=object_color_ref
                disabled=Signal::derive(move || !has_selected_objects())
                knob_style=Signal::derive(object_color_knob_style)
                color_value=Signal::derive(object_color_base)
                on_pointer_down=on_object_color_pointer_down
                on_pointer_move=on_object_color_pointer_move
                on_pointer_up=on_object_color_pointer_up
                on_center_pointer_down=on_object_color_readout_pointer_down
                on_color_input=on_object_color_input
                on_reset_click=on_object_color_reset
            />
            <ColorDial
                class="canvas-object-border"
                disabled_class="canvas-object-border--disabled"
                title="Drag to set selected border thickness; center picks border color"
                swatch_title="Selected border color"
                reset_title="Reset border to defaults"
                center_label=Signal::derive(move || format_border_width_label(object_border_width()))
                knob_class="canvas-object-border__knob"
                node_ref=object_border_ref
                disabled=Signal::derive(move || !has_selected_objects())
                knob_style=Signal::derive(object_border_knob_style)
                color_value=Signal::derive(object_border_color)
                on_pointer_down=on_object_border_pointer_down
                on_pointer_move=on_object_border_pointer_move
                on_pointer_up=on_object_border_pointer_up
                on_center_pointer_down=on_object_border_readout_pointer_down
                on_color_input=on_object_border_input
                on_reset_click=on_object_border_reset
            />
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
fn set_camera_view(engine: &mut Engine, center_x: f64, center_y: f64, zoom: f64, rotation_deg: f64) {
    let clamped_zoom = zoom.clamp(0.1, 10.0);
    engine.core.camera.zoom = clamped_zoom;
    engine.set_view_rotation_deg(rotation_deg);
    engine.core.camera.pan_x = (engine.core.viewport_width * 0.5) - (center_x * clamped_zoom);
    engine.core.camera.pan_y = (engine.core.viewport_height * 0.5) - (center_y * clamped_zoom);
}

#[cfg(feature = "hydrate")]
fn zoom_view_preserving_center(engine: &mut Engine, zoom: f64) {
    let center_screen = viewport_center_screen(engine);
    let center_world = engine.camera().screen_to_world(center_screen, center_screen);
    let rotation = engine.view_rotation_deg();
    set_camera_view(engine, center_world.x, center_world.y, zoom, rotation);
}

#[cfg(feature = "hydrate")]
const ZOOM_DIAL_MIN_ANGLE_DEG: f64 = -135.0;
#[cfg(feature = "hydrate")]
const ZOOM_DIAL_MAX_ANGLE_DEG: f64 = 135.0;
#[cfg(feature = "hydrate")]
const ZOOM_DIAL_TICK_TENSION_RANGE_DEG: f64 = 14.0;
#[cfg(feature = "hydrate")]
const ZOOM_DIAL_TICK_TENSION_STRENGTH: f64 = 0.42;

#[cfg(feature = "hydrate")]
fn zoom_angle_from_pointer(ev: &leptos::ev::PointerEvent, element: &web_sys::HtmlDivElement) -> Option<f64> {
    let rect = element.get_bounding_client_rect();
    let cx = rect.x() + (rect.width() * 0.5);
    let cy = rect.y() + (rect.height() * 0.5);
    let dx = f64::from(ev.client_x()) - cx;
    let dy = f64::from(ev.client_y()) - cy;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return None;
    }

    let raw_top_based = normalize_degrees_360(dy.atan2(dx).to_degrees() + 90.0);
    let signed = if raw_top_based > 180.0 {
        raw_top_based - 360.0
    } else {
        raw_top_based
    };
    let clamped = signed.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    Some(apply_zoom_tick_tension(clamped))
}

#[cfg(feature = "hydrate")]
fn apply_zoom_tick_tension(angle: f64) -> f64 {
    let ticks = [
        ZOOM_DIAL_MIN_ANGLE_DEG,
        -90.0,
        -45.0,
        0.0,
        45.0,
        90.0,
        ZOOM_DIAL_MAX_ANGLE_DEG,
    ];
    let mut adjusted = angle;
    for tick in ticks {
        let distance = (adjusted - tick).abs();
        if distance >= ZOOM_DIAL_TICK_TENSION_RANGE_DEG {
            continue;
        }
        let weight = 1.0 - (distance / ZOOM_DIAL_TICK_TENSION_RANGE_DEG);
        adjusted += (tick - adjusted) * weight * ZOOM_DIAL_TICK_TENSION_STRENGTH;
    }
    adjusted.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

#[cfg(feature = "hydrate")]
fn dial_angle_from_zoom(zoom: f64) -> f64 {
    ((zoom - 1.0) * 180.0)
        .clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

#[cfg(not(feature = "hydrate"))]
fn dial_angle_from_zoom(_zoom: f64) -> f64 {
    0.0
}

#[cfg(feature = "hydrate")]
fn zoom_from_dial_angle(angle: f64) -> f64 {
    (1.0 + (angle / 180.0)).clamp(0.1, 10.0)
}

#[cfg(feature = "hydrate")]
fn dial_angle_from_color_shift(shift: f64) -> f64 {
    (shift.clamp(-1.0, 1.0) * ZOOM_DIAL_MAX_ANGLE_DEG).clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

#[cfg(not(feature = "hydrate"))]
fn dial_angle_from_color_shift(_shift: f64) -> f64 {
    0.0
}

#[cfg(feature = "hydrate")]
fn color_shift_from_dial_angle(angle: f64) -> f64 {
    (angle / ZOOM_DIAL_MAX_ANGLE_DEG).clamp(-1.0, 1.0)
}

#[cfg(feature = "hydrate")]
const BORDER_WIDTH_MIN: f64 = 0.0;
#[cfg(feature = "hydrate")]
const BORDER_WIDTH_MAX: f64 = 24.0;

#[cfg(feature = "hydrate")]
fn dial_angle_from_border_width(width: f64) -> f64 {
    let clamped = width.clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    let t = if BORDER_WIDTH_MAX <= BORDER_WIDTH_MIN {
        0.0
    } else {
        (clamped - BORDER_WIDTH_MIN) / (BORDER_WIDTH_MAX - BORDER_WIDTH_MIN)
    };
    ZOOM_DIAL_MIN_ANGLE_DEG + (t * (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG))
}

#[cfg(not(feature = "hydrate"))]
fn dial_angle_from_border_width(_width: f64) -> f64 {
    0.0
}

#[cfg(feature = "hydrate")]
fn border_width_from_dial_angle(angle: f64) -> f64 {
    let clamped_angle = angle.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    let t = (clamped_angle - ZOOM_DIAL_MIN_ANGLE_DEG) / (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG);
    snap_border_width_to_px(BORDER_WIDTH_MIN + (t * (BORDER_WIDTH_MAX - BORDER_WIDTH_MIN)))
}

#[cfg(feature = "hydrate")]
fn format_border_width_label(width: f64) -> String {
    let rounded = width.round();
    if (width - rounded).abs() < 0.05 {
        format!("{}px", rounded as i64)
    } else {
        format!("{width:.1}px")
    }
}

#[cfg(not(feature = "hydrate"))]
fn format_border_width_label(_width: f64) -> String {
    "1px".to_owned()
}

#[cfg(feature = "hydrate")]
fn snap_border_width_to_px(width: f64) -> f64 {
    width.round().clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}


#[cfg(feature = "hydrate")]
fn send_cursor_presence_if_needed(
    engine: &Engine,
    board: RwSignal<BoardState>,
    auth: RwSignal<AuthState>,
    sender: RwSignal<FrameSender>,
    last_sent_ms: RwSignal<f64>,
    last_sent_view: RwSignal<Option<(f64, f64, f64, f64)>>,
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
    const CAMERA_ONLY_MIN_INTERVAL_MS: f64 = 40.0;
    const CAMERA_CENTER_DEADBAND_WORLD: f64 = 0.2;
    const CAMERA_ZOOM_DEADBAND: f64 = 0.001;
    const CAMERA_ROTATION_DEADBAND_DEG: f64 = 0.1;

    let now = now_ms();
    if !force && !has_cursor_point && now - last_sent_ms.get_untracked() < CAMERA_ONLY_MIN_INTERVAL_MS {
        return;
    }
    let Some(board_id) = state.board_id else {
        return;
    };

    let camera = engine.camera();
    let center_screen = viewport_center_screen(engine);
    let center_world = camera.screen_to_world(center_screen, center_screen);
    let center_x = center_world.x;
    let center_y = center_world.y;
    let zoom = camera.zoom;
    let rotation = camera.view_rotation_deg;
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p, center_screen));

    if !force
        && !has_cursor_point
        && let Some((last_x, last_y, last_zoom, last_rotation)) = last_sent_view.get_untracked()
    {
        let dx = center_x - last_x;
        let dy = center_y - last_y;
        let center_dist = (dx * dx + dy * dy).sqrt();
        let zoom_delta = (zoom - last_zoom).abs();
        let rotation_delta = (rotation - last_rotation).abs();
        if center_dist < CAMERA_CENTER_DEADBAND_WORLD
            && zoom_delta < CAMERA_ZOOM_DEADBAND
            && rotation_delta < CAMERA_ROTATION_DEADBAND_DEG
        {
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
            "camera_rotation": rotation,
        }),
    };
    if sender.get_untracked().send(&frame) {
        last_sent_ms.set(now);
        last_sent_view.set(Some((center_x, center_y, zoom, rotation)));
    }
}

#[cfg(feature = "hydrate")]
fn map_tool(tool: ToolType) -> CanvasTool {
    match tool {
        ToolType::Select => CanvasTool::Select,
        ToolType::Sticky | ToolType::Rectangle | ToolType::Frame | ToolType::Youtube => CanvasTool::Select,
        ToolType::Ellipse => CanvasTool::Ellipse,
        ToolType::Line | ToolType::Connector => CanvasTool::Line,
        ToolType::Text => CanvasTool::Text,
        ToolType::Draw | ToolType::Eraser => CanvasTool::Select,
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
fn viewport_center_screen(engine: &Engine) -> CanvasPoint {
    CanvasPoint::new(engine.core.viewport_width * 0.5, engine.core.viewport_height * 0.5)
}

fn normalize_degrees_360(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

#[cfg(feature = "hydrate")]
fn signed_angle_delta_deg(current: f64, start: f64) -> f64 {
    let mut delta = current - start;
    while delta > 180.0 {
        delta -= 360.0;
    }
    while delta < -180.0 {
        delta += 360.0;
    }
    delta
}

#[cfg(feature = "hydrate")]
fn angular_delta_deg(a: f64, b: f64) -> f64 {
    let delta = (a - b).abs().rem_euclid(360.0);
    delta.min(360.0 - delta)
}

#[cfg(feature = "hydrate")]
fn apply_compass_drag_snapping(raw_deg: f64, shift_snap: bool) -> f64 {
    const CARDINAL_SNAP_EPS_DEG: f64 = 6.0;
    const SHIFT_STEP_DEG: f64 = 15.0;

    let mut deg = normalize_degrees_360(raw_deg);
    for target in [0.0, 90.0, 180.0, 270.0] {
        if angular_delta_deg(deg, target) <= CARDINAL_SNAP_EPS_DEG {
            deg = target;
            break;
        }
    }
    if shift_snap {
        deg = (deg / SHIFT_STEP_DEG).round() * SHIFT_STEP_DEG;
    }
    normalize_degrees_360(deg)
}

#[cfg(feature = "hydrate")]
fn compass_angle_from_pointer(ev: &leptos::ev::PointerEvent, element: &web_sys::HtmlDivElement) -> Option<f64> {
    let rect = element.get_bounding_client_rect();
    let cx = rect.x() + (rect.width() * 0.5);
    let cy = rect.y() + (rect.height() * 0.5);
    let dx = f64::from(ev.client_x()) - cx;
    let dy = f64::from(ev.client_y()) - cy;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return None;
    }

    Some(normalize_degrees_360(dy.atan2(dx).to_degrees() + 90.0))
}

#[cfg(feature = "hydrate")]
fn pointer_event_hits_control(ev: &leptos::ev::PointerEvent, selector: &str) -> bool {
    use wasm_bindgen::JsCast;

    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(selector).ok().flatten())
        .is_some()
}

#[cfg(feature = "hydrate")]
fn has_selection(board: RwSignal<BoardState>) -> bool {
    !board.get_untracked().selection.is_empty()
}

#[cfg(feature = "hydrate")]
fn selected_object_rotations(board: RwSignal<BoardState>) -> Vec<(String, f64)> {
    let state = board.get_untracked();
    state
        .selection
        .iter()
        .filter_map(|id| state.objects.get(id).map(|obj| (id.clone(), obj.rotation)))
        .collect()
}

#[cfg(feature = "hydrate")]
fn selection_scale_seed(board: RwSignal<BoardState>) -> Option<SelectionScaleDragState> {
    let state = board.get_untracked();
    let mut items: Vec<SelectionScaleSeed> = Vec::new();
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        let width = obj.width.unwrap_or(120.0).max(1.0);
        let height = obj.height.unwrap_or(80.0).max(1.0);
        let (base_width, base_height, start_scale) = object_scale_components(obj, width, height);
        let x = obj.x;
        let y = obj.y;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
        items.push(SelectionScaleSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            base_width,
            base_height,
            start_scale,
            x,
            y,
            width,
            height,
        });
    }
    if items.is_empty() {
        return None;
    }
    let start_group_scale = selection_representative_scale_from_items(&items);
    Some(SelectionScaleDragState {
        start_items: items,
        group_center_x: (min_x + max_x) * 0.5,
        group_center_y: (min_y + max_y) * 0.5,
        start_group_scale,
    })
}

#[cfg(feature = "hydrate")]
fn apply_selection_scale_drag(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionScaleDragState>>,
    target_scale: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let target_scale = target_scale.clamp(0.1, 10.0);
    let multiplier = if drag_state.start_group_scale.abs() < f64::EPSILON {
        1.0
    } else {
        target_scale / drag_state.start_group_scale
    };
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            let start_cx = seed.x + (seed.width * 0.5);
            let start_cy = seed.y + (seed.height * 0.5);
            let next_scale = (seed.start_scale * multiplier).clamp(0.1, 10.0);
            let new_w = (seed.base_width * next_scale).max(1.0);
            let new_h = (seed.base_height * next_scale).max(1.0);
            let new_cx = drag_state.group_center_x + ((start_cx - drag_state.group_center_x) * multiplier);
            let new_cy = drag_state.group_center_y + ((start_cy - drag_state.group_center_y) * multiplier);
            obj.width = Some(new_w);
            obj.height = Some(new_h);
            obj.x = new_cx - (new_w * 0.5);
            obj.y = new_cy - (new_h * 0.5);
            upsert_object_scale_props(obj, next_scale, seed.base_width, seed.base_height);
        }
    });
}

#[cfg(feature = "hydrate")]
fn commit_selection_scale_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionScaleDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let changed = (obj.x - seed.x).abs() > 0.01
            || (obj.y - seed.y).abs() > 0.01
            || (obj.width.unwrap_or(seed.width) - seed.width).abs() > 0.01
            || (obj.height.unwrap_or(seed.height) - seed.height).abs() > 0.01;
        if !changed {
            continue;
        }
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(seed.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": seed.id,
                "version": seed.version,
                "x": obj.x,
                "y": obj.y,
                "width": obj.width.unwrap_or(seed.width),
                "height": obj.height.unwrap_or(seed.height),
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
    drag_state_signal.set(None);
}

#[cfg(feature = "hydrate")]
fn apply_group_scale_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, target_scale: f64) {
    let target_scale = target_scale.clamp(0.1, 10.0);
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }
    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let width = obj.width.unwrap_or(120.0).max(1.0);
            let height = obj.height.unwrap_or(80.0).max(1.0);
            let (base_width, base_height, _current_scale) = object_scale_components(obj, width, height);
            let cx = obj.x + (width * 0.5);
            let cy = obj.y + (height * 0.5);
            let new_w = (base_width * target_scale).max(1.0);
            let new_h = (base_height * target_scale).max(1.0);
            obj.width = Some(new_w);
            obj.height = Some(new_h);
            obj.x = cx - (new_w * 0.5);
            obj.y = cy - (new_h * 0.5);
            upsert_object_scale_props(obj, target_scale, base_width, base_height);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "x": obj.x,
                "y": obj.y,
                "width": obj.width.unwrap_or(120.0),
                "height": obj.height.unwrap_or(80.0),
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
}

#[cfg(not(feature = "hydrate"))]
fn apply_group_scale_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _target_scale: f64) {}

#[cfg(feature = "hydrate")]
fn object_scale_components(obj: &crate::net::types::BoardObject, width: f64, height: f64) -> (f64, f64, f64) {
    let base_width = obj
        .props
        .get("baseWidth")
        .and_then(value_as_f64)
        .unwrap_or(width)
        .max(1.0);
    let base_height = obj
        .props
        .get("baseHeight")
        .and_then(value_as_f64)
        .unwrap_or(height)
        .max(1.0);
    let scale = obj
        .props
        .get("scale")
        .and_then(value_as_f64)
        .unwrap_or_else(|| (width / base_width).clamp(0.1, 10.0))
        .clamp(0.1, 10.0);
    (base_width, base_height, scale)
}

#[cfg(feature = "hydrate")]
fn upsert_object_scale_props(obj: &mut crate::net::types::BoardObject, scale: f64, base_width: f64, base_height: f64) {
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("scale".to_owned(), serde_json::json!(scale));
        map.insert("baseWidth".to_owned(), serde_json::json!(base_width));
        map.insert("baseHeight".to_owned(), serde_json::json!(base_height));
    }
}

#[cfg(feature = "hydrate")]
fn value_as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))
}

#[cfg(feature = "hydrate")]
fn reset_scale_props_baseline(props: &mut serde_json::Value, width: f64, height: f64) {
    if !props.is_object() {
        *props = serde_json::json!({});
    }
    if let Some(map) = props.as_object_mut() {
        map.insert("scale".to_owned(), serde_json::Value::Null);
        map.insert("baseWidth".to_owned(), serde_json::json!(width.max(1.0)));
        map.insert("baseHeight".to_owned(), serde_json::json!(height.max(1.0)));
    }
}

#[cfg(feature = "hydrate")]
fn reset_wire_object_scale_baseline(obj: &mut crate::net::types::BoardObject) {
    let width = obj.width.unwrap_or(120.0).max(1.0);
    let height = obj.height.unwrap_or(80.0).max(1.0);
    reset_scale_props_baseline(&mut obj.props, width, height);
}

#[cfg(feature = "hydrate")]
fn selection_representative_base_color_hex(board: RwSignal<BoardState>) -> String {
    let state = board.get();
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_base_fill_hex))
        .unwrap_or_else(|| "#D94B4B".to_owned())
}

#[cfg(not(feature = "hydrate"))]
fn selection_representative_base_color_hex(_board: RwSignal<BoardState>) -> String {
    "#D94B4B".to_owned()
}

#[cfg(feature = "hydrate")]
fn selection_representative_lightness_shift(board: RwSignal<BoardState>) -> f64 {
    let state = board.get();
    let mut shifts: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        shifts.push(object_lightness_shift(obj));
    }
    if shifts.is_empty() {
        return 0.0;
    }
    (shifts.iter().sum::<f64>() / shifts.len() as f64).clamp(-1.0, 1.0)
}

#[cfg(not(feature = "hydrate"))]
fn selection_representative_lightness_shift(_board: RwSignal<BoardState>) -> f64 {
    0.0
}

#[cfg(feature = "hydrate")]
fn selection_color_seed(board: RwSignal<BoardState>) -> Option<SelectionColorDragState> {
    let state = board.get_untracked();
    let mut items = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        items.push(SelectionColorSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            start_fill: object_fill_hex(obj),
            start_base_fill: object_base_fill_hex(obj),
            start_lightness_shift: object_lightness_shift(obj),
        });
    }
    if items.is_empty() {
        return None;
    }
    Some(SelectionColorDragState { start_items: items })
}

#[cfg(feature = "hydrate")]
fn apply_selection_color_shift(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionColorDragState>>,
    target_shift: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let shift = target_shift.clamp(-1.0, 1.0);
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            upsert_object_color_props(obj, &seed.start_base_fill, shift);
        }
    });
}

#[cfg(feature = "hydrate")]
fn commit_selection_color_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionColorDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let fill = object_fill_hex(obj);
        let base_fill = object_base_fill_hex(obj);
        let shift = object_lightness_shift(obj);
        let changed = fill != seed.start_fill
            || base_fill != seed.start_base_fill
            || (shift - seed.start_lightness_shift).abs() > 0.001;
        if !changed {
            continue;
        }
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(seed.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": seed.id,
                "version": seed.version,
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
    drag_state_signal.set(None);
}

#[cfg(feature = "hydrate")]
fn apply_group_base_color_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, raw_color: String) {
    let base_fill = normalize_hex_color(raw_color, "#D94B4B");
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let shift = object_lightness_shift(obj);
            upsert_object_color_props(obj, &base_fill, shift);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
}

#[cfg(feature = "hydrate")]
fn apply_group_background_defaults_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let base_fill = "#D94B4B".to_owned();
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            upsert_object_color_props(obj, &base_fill, 0.0);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
}

#[cfg(not(feature = "hydrate"))]
#[allow(dead_code)]
fn apply_group_base_color_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _raw_color: String) {}

#[cfg(not(feature = "hydrate"))]
#[allow(dead_code)]
fn apply_group_background_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

#[cfg(feature = "hydrate")]
fn upsert_object_color_props(obj: &mut crate::net::types::BoardObject, base_fill: &str, lightness_shift: f64) {
    let base = normalize_hex_color(base_fill.to_owned(), "#D94B4B");
    let shift = lightness_shift.clamp(-1.0, 1.0);
    let fill = apply_lightness_shift_to_hex(&base, shift);
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("baseFill".to_owned(), serde_json::Value::String(base));
        map.insert("lightnessShift".to_owned(), serde_json::json!(shift));
        map.insert("fill".to_owned(), serde_json::Value::String(fill.clone()));
        map.insert("backgroundColor".to_owned(), serde_json::Value::String(fill));
    }
}

#[cfg(feature = "hydrate")]
fn object_fill_hex(obj: &crate::net::types::BoardObject) -> String {
    obj.props
        .get("fill")
        .and_then(|v| v.as_str())
        .or_else(|| obj.props.get("backgroundColor").and_then(|v| v.as_str()))
        .or_else(|| obj.props.get("borderColor").and_then(|v| v.as_str()))
        .map(|s| normalize_hex_color(s.to_owned(), "#D94B4B"))
        .unwrap_or_else(|| "#D94B4B".to_owned())
}

#[cfg(feature = "hydrate")]
fn object_base_fill_hex(obj: &crate::net::types::BoardObject) -> String {
    obj.props
        .get("baseFill")
        .and_then(|v| v.as_str())
        .map(|s| normalize_hex_color(s.to_owned(), "#D94B4B"))
        .unwrap_or_else(|| object_fill_hex(obj))
}

#[cfg(feature = "hydrate")]
fn object_lightness_shift(obj: &crate::net::types::BoardObject) -> f64 {
    obj.props
        .get("lightnessShift")
        .and_then(value_as_f64)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0)
}

#[cfg(feature = "hydrate")]
fn normalize_hex_color(value: String, fallback: &str) -> String {
    let fallback_rgb = parse_hex_rgb(fallback).unwrap_or((217, 75, 75));
    let (r, g, b) = parse_hex_rgb(&value).unwrap_or(fallback_rgb);
    format!("#{r:02X}{g:02X}{b:02X}")
}

#[cfg(feature = "hydrate")]
fn parse_hex_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hex = &trimmed[1..];
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

#[cfg(feature = "hydrate")]
fn apply_lightness_shift_to_hex(base_hex: &str, shift: f64) -> String {
    let (r, g, b) = parse_hex_rgb(base_hex).unwrap_or((217, 75, 75));
    let shift = shift.clamp(-1.0, 1.0);
    let scale = |channel: u8| -> u8 {
        let current = f64::from(channel);
        let adjusted = if shift >= 0.0 {
            current + ((255.0 - current) * shift)
        } else {
            current * (1.0 + shift)
        };
        adjusted.round().clamp(0.0, 255.0) as u8
    };
    format!("#{:02X}{:02X}{:02X}", scale(r), scale(g), scale(b))
}

#[cfg(feature = "hydrate")]
fn selection_representative_border_color_hex(board: RwSignal<BoardState>) -> String {
    let state = board.get();
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_border_color_hex))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

#[cfg(not(feature = "hydrate"))]
fn selection_representative_border_color_hex(_board: RwSignal<BoardState>) -> String {
    "#1F1A17".to_owned()
}

#[cfg(feature = "hydrate")]
fn selection_representative_border_width(board: RwSignal<BoardState>) -> f64 {
    let state = board.get();
    let mut widths: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        widths.push(object_border_width(obj));
    }
    if widths.is_empty() {
        return 1.0;
    }
    (widths.iter().sum::<f64>() / widths.len() as f64).clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

#[cfg(not(feature = "hydrate"))]
fn selection_representative_border_width(_board: RwSignal<BoardState>) -> f64 {
    1.0
}

#[cfg(feature = "hydrate")]
fn selection_border_seed(board: RwSignal<BoardState>) -> Option<SelectionBorderDragState> {
    let state = board.get_untracked();
    let mut items = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        items.push(SelectionBorderSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            start_border_color: object_border_color_hex(obj),
            start_border_width: object_border_width(obj),
        });
    }
    if items.is_empty() {
        return None;
    }
    Some(SelectionBorderDragState { start_items: items })
}

#[cfg(feature = "hydrate")]
fn apply_selection_border_width(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionBorderDragState>>,
    target_width: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let width = target_width.clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            upsert_object_border_props(obj, &seed.start_border_color, width);
        }
    });
}

#[cfg(feature = "hydrate")]
fn commit_selection_border_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionBorderDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let color = object_border_color_hex(obj);
        let width = object_border_width(obj);
        let changed = color != seed.start_border_color || (width - seed.start_border_width).abs() > 0.001;
        if !changed {
            continue;
        }
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(seed.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": seed.id,
                "version": seed.version,
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
    drag_state_signal.set(None);
}

#[cfg(feature = "hydrate")]
fn apply_group_border_color_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, raw_color: String) {
    let border = normalize_hex_color(raw_color, "#1F1A17");
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let width = object_border_width(obj);
            upsert_object_border_props(obj, &border, width);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
}

#[cfg(feature = "hydrate")]
fn apply_group_border_defaults_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let border = "#1F1A17".to_owned();
    let width = 0.0;
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            upsert_object_border_props(obj, &border, width);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "props": obj.props,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
}

#[cfg(not(feature = "hydrate"))]
#[allow(dead_code)]
fn apply_group_border_color_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _raw_color: String) {}

#[cfg(not(feature = "hydrate"))]
#[allow(dead_code)]
fn apply_group_border_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

#[cfg(feature = "hydrate")]
fn upsert_object_border_props(obj: &mut crate::net::types::BoardObject, border_color: &str, border_width: f64) {
    let color = normalize_hex_color(border_color.to_owned(), "#1F1A17");
    let width = snap_border_width_to_px(border_width);
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("borderColor".to_owned(), serde_json::Value::String(color.clone()));
        map.insert("stroke".to_owned(), serde_json::Value::String(color));
        map.insert("borderWidth".to_owned(), serde_json::json!(width));
        map.insert("stroke_width".to_owned(), serde_json::json!(width));
    }
}

#[cfg(feature = "hydrate")]
fn object_border_color_hex(obj: &crate::net::types::BoardObject) -> String {
    obj.props
        .get("borderColor")
        .and_then(|v| v.as_str())
        .or_else(|| obj.props.get("stroke").and_then(|v| v.as_str()))
        .or_else(|| obj.props.get("fill").and_then(|v| v.as_str()))
        .map(|s| normalize_hex_color(s.to_owned(), "#1F1A17"))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

#[cfg(feature = "hydrate")]
fn object_border_width(obj: &crate::net::types::BoardObject) -> f64 {
    obj.props
        .get("borderWidth")
        .and_then(value_as_f64)
        .or_else(|| obj.props.get("stroke_width").and_then(value_as_f64))
        .unwrap_or(0.0)
        .clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

#[cfg(feature = "hydrate")]
fn selection_representative_rotation_deg(board: RwSignal<BoardState>) -> f64 {
    let state = board.get();
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut count = 0_usize;
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        let r = obj.rotation.to_radians();
        sum_x += r.cos();
        sum_y += r.sin();
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    normalize_degrees_360(sum_y.atan2(sum_x).to_degrees())
}

#[cfg(not(feature = "hydrate"))]
fn selection_representative_rotation_deg(_board: RwSignal<BoardState>) -> f64 {
    0.0
}

#[cfg(feature = "hydrate")]
fn selection_representative_scale_factor(board: RwSignal<BoardState>) -> f64 {
    let state = board.get();
    let mut scales: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        let width = obj.width.unwrap_or(120.0).max(1.0);
        let height = obj.height.unwrap_or(80.0).max(1.0);
        let (_base_w, _base_h, scale) = object_scale_components(obj, width, height);
        scales.push(scale);
    }
    if scales.is_empty() {
        return 1.0;
    }
    scales.iter().sum::<f64>() / scales.len() as f64
}

#[cfg(not(feature = "hydrate"))]
fn selection_representative_scale_factor(_board: RwSignal<BoardState>) -> f64 {
    1.0
}

#[cfg(feature = "hydrate")]
fn selection_representative_scale_from_items(items: &[SelectionScaleSeed]) -> f64 {
    if items.is_empty() {
        return 1.0;
    }
    items.iter().map(|s| s.start_scale).sum::<f64>() / items.len() as f64
}

#[cfg(feature = "hydrate")]
fn apply_selection_rotation_drag(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionRotationDragState>>,
    pointer_angle_deg: f64,
    shift_snap: bool,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let snapped_pointer = apply_compass_drag_snapping(pointer_angle_deg, shift_snap);
    let delta = signed_angle_delta_deg(snapped_pointer, drag_state.start_pointer_angle_deg);
    board.update(|b| {
        for (id, start_rotation) in &drag_state.start_rotations {
            if let Some(obj) = b.objects.get_mut(id) {
                obj.rotation = normalize_degrees_360(*start_rotation + delta);
            }
        }
    });
}

#[cfg(feature = "hydrate")]
fn commit_selection_rotation_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionRotationDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for (id, start_rotation) in &drag_state.start_rotations {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        if angular_delta_deg(obj.rotation, *start_rotation) < 0.01 {
            continue;
        }
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "rotation": obj.rotation,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
    drag_state_signal.set(None);
}

#[cfg(feature = "hydrate")]
fn apply_group_rotation_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, target_deg: f64) {
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    let current = selection_representative_rotation_deg(board);
    let delta = signed_angle_delta_deg(target_deg, current);

    board.update(|b| {
        for id in &selected {
            if let Some(obj) = b.objects.get_mut(id) {
                obj.rotation = normalize_degrees_360(obj.rotation + delta);
            }
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(obj.board_id.clone()),
            from: None,
            syscall: "object:update".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "id": obj.id,
                "version": obj.version,
                "rotation": obj.rotation,
            }),
        };
        let _ = sender.get_untracked().send(&frame);
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
fn update_youtube_overlay_from_click(
    engine: &Engine,
    point_screen: CanvasPoint,
    ev: &leptos::ev::PointerEvent,
    active_youtube: RwSignal<Option<(String, String)>>,
) {
    if ev.button() != 0 {
        return;
    }
    let center = viewport_center_screen(engine);
    let world = engine.camera().screen_to_world(point_screen, center);
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
    let camera_center_screen = viewport_center_screen(engine);
    let camera_center_world = camera.screen_to_world(camera_center_screen, camera_center_screen);
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p, camera_center_screen));
    let sample_ms = now_ms();

    canvas_view.update(|v| {
        if let Some(prev_sample_ms) = v.fps_last_sample_ms {
            let dt_ms = sample_ms - prev_sample_ms;
            if (5.0..2000.0).contains(&dt_ms) {
                let instantaneous_fps = 1000.0 / dt_ms;
                let ema_alpha = 0.2;
                v.fps = Some(v.fps.map_or(instantaneous_fps, |prev| {
                    (prev * (1.0 - ema_alpha)) + (instantaneous_fps * ema_alpha)
                }));
            }
        }
        v.fps_last_sample_ms = Some(sample_ms);
        v.cursor_world = cursor_world.map(|p| WirePoint { x: p.x, y: p.y });
        v.camera_center_world = WirePoint { x: camera_center_world.x, y: camera_center_world.y };
        v.zoom = camera.zoom;
        v.pan_x = camera.pan_x;
        v.pan_y = camera.pan_y;
        v.view_rotation_deg = camera.view_rotation_deg;
        v.viewport_width = engine.core.viewport_width;
        v.viewport_height = engine.core.viewport_height;
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
                "borderWidth": 0
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
                "borderWidth": 0
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
                "borderWidth": 0,
                "stroke": "#1F1A17",
                "stroke_width": 0
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
                "borderWidth": 0
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
                "borderWidth": 0
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
                "borderWidth": 0
            }),
        )),
        ToolType::Text => Some((
            "text",
            220.0,
            56.0,
            serde_json::json!({
                "text": "Text",
                "fontSize": 24
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
        ToolType::Text => Some((220.0, 56.0, "rgba(217, 75, 75, 0.22)")),
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

    let center = viewport_center_screen(engine);
    let world = engine.camera().screen_to_world(point_screen, center);
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
        "text" => CanvasKind::Text,
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
        let color = map
            .get("color")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);
        if let Some(v) = map
            .get("backgroundColor")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
        {
            map.entry("fill".to_owned())
                .or_insert_with(|| serde_json::Value::String(v));
        } else if let Some(v) = color.clone() {
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
        } else if let Some(v) = color {
            map.entry("stroke".to_owned())
                .or_insert_with(|| serde_json::Value::String(v));
        }
        if let Some(v) = map
            .get("borderWidth")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        {
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
                let geometry_changed = fields.width.is_some() || fields.height.is_some();
                if let Some(obj) = engine.object(&id)
                    && let Some(mut local) = to_wire_object(obj, &board_id)
                {
                    if geometry_changed {
                        reset_wire_object_scale_baseline(&mut local);
                    }
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
                if geometry_changed
                    && let Some(obj) = engine.object(&id)
                {
                    let mut props = obj.props.clone();
                    reset_scale_props_baseline(&mut props, obj.width, obj.height);
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
        CanvasKind::Text => "text",
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
