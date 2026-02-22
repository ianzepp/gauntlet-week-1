//! Bridge component between Leptos state and the imperative `canvas::Engine`.
//!
//! ARCHITECTURE
//! ============
//! The canvas crate owns render-time performance concerns while this host maps
//! websocket/state events into engine operations and publishes viewport telemetry.

use leptos::prelude::*;
use leptos::tachys::view::any_view::IntoAny;

use crate::app::FrameSender;
use crate::components::dial::{ColorDial, CompassDial, ZoomDial};
#[cfg(feature = "hydrate")]
use crate::net::types::{BoardObject, Frame, FrameStatus};
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::canvas_view::CanvasViewState;
#[cfg(feature = "hydrate")]
use crate::state::ui::ToolType;
use crate::state::ui::{UiState, ViewMode};
#[cfg(feature = "hydrate")]
use crate::util::animation::{project_clip_scene, resolve_active_clip};
#[cfg(feature = "hydrate")]
use crate::util::canvas_input::{
    compass_angle_from_pointer, map_button, map_modifiers, map_tool, pointer_event_hits_control, pointer_point,
    should_prevent_default_key, wheel_point, zoom_angle_from_pointer,
};
#[cfg(feature = "hydrate")]
use crate::util::canvas_viewport::{
    center_world_origin, now_ms, send_cursor_presence_if_needed, set_camera_view, sync_canvas_view_state,
    sync_viewport, viewport_center_screen, zoom_view_preserving_center,
};
#[allow(unused_imports)]
use crate::util::dial_math::{
    BORDER_WIDTH_MAX, BORDER_WIDTH_MIN, TEXT_SIZE_MAX, TEXT_SIZE_MIN, ZOOM_DIAL_MAX_ANGLE_DEG, ZOOM_DIAL_MIN_ANGLE_DEG,
    angular_delta_deg, apply_compass_drag_snapping, apply_zoom_tick_tension, border_width_from_dial_angle,
    color_shift_from_dial_angle, dial_angle_from_border_width, dial_angle_from_color_shift, dial_angle_from_font_size,
    dial_angle_from_zoom, font_size_from_dial_angle, format_border_width_label, format_text_size_label,
    normalize_degrees_360, signed_angle_delta_deg, snap_border_width_to_px, snap_font_size_to_px, zoom_from_dial_angle,
};
#[cfg(feature = "hydrate")]
use crate::util::object_props::{reset_scale_props_baseline, reset_wire_object_scale_baseline};
use crate::util::selection_actions::apply_group_scale_target;
#[cfg(feature = "hydrate")]
use crate::util::selection_actions::{
    SelectionBorderDragState, SelectionColorDragState, SelectionRotationDragState, SelectionScaleDragState,
    SelectionTextStyleDragState, apply_group_rotation_target, apply_selection_border_width,
    apply_selection_color_shift, apply_selection_font_size, apply_selection_rotation_drag, apply_selection_scale_drag,
    commit_selection_border_updates, commit_selection_color_updates, commit_selection_rotation_updates,
    commit_selection_scale_updates, commit_selection_text_style_updates, has_selection, selected_object_rotations,
    selection_border_seed, selection_color_seed, selection_scale_seed, selection_text_style_seed,
};
#[allow(unused_imports)]
use crate::util::selection_actions::{
    apply_group_background_defaults_target, apply_group_base_color_target, apply_group_border_color_target,
    apply_group_border_defaults_target, apply_group_text_color_target, apply_group_text_style_defaults_target,
};
use crate::util::selection_metrics::{
    representative_base_color_hex, representative_border_color_hex, representative_border_width,
    representative_font_size, representative_lightness_shift, representative_rotation_deg, representative_scale_factor,
    representative_text_color_hex,
};
#[cfg(feature = "hydrate")]
use crate::util::shape_palette::{materialize_shape_props, placement_shape};

#[cfg(feature = "hydrate")]
use gloo_timers::callback::{Interval, Timeout};
#[cfg(feature = "hydrate")]
use std::cell::RefCell;
#[cfg(feature = "hydrate")]
use std::collections::HashMap;
#[cfg(feature = "hydrate")]
use std::collections::hash_map::DefaultHasher;
#[cfg(feature = "hydrate")]
use std::hash::{Hash, Hasher};
#[cfg(feature = "hydrate")]
use std::rc::Rc;

#[cfg(feature = "hydrate")]
use canvas::camera::Point as CanvasPoint;
#[cfg(feature = "hydrate")]
use canvas::doc::{BoardObject as CanvasObject, ObjectKind as CanvasKind};
#[cfg(feature = "hydrate")]
use canvas::engine::{Action, Engine};
#[cfg(feature = "hydrate")]
use canvas::input::{InputState as CanvasInputState, Key as CanvasKey, WheelDelta};
#[cfg(feature = "hydrate")]
use js_sys::Date;
#[cfg(feature = "hydrate")]
use wasm_bindgen::{JsCast, closure::Closure};

#[cfg(feature = "hydrate")]
fn render_and_track(engine: &mut Engine, canvas_view: RwSignal<CanvasViewState>) {
    let started_ms = Date::now();
    let _ = engine.render();
    let elapsed_ms = (Date::now() - started_ms).max(0.0);
    canvas_view.update(|view| {
        view.last_render_ms = Some(elapsed_ms);
    });
}

#[cfg(feature = "hydrate")]
fn request_render(
    engine: &Rc<RefCell<Option<Engine>>>,
    canvas_view: RwSignal<CanvasViewState>,
    raf_pending: RwSignal<bool>,
) {
    if raf_pending.get_untracked() {
        return;
    }
    raf_pending.set(true);

    let Some(window) = web_sys::window() else {
        raf_pending.set(false);
        if let Some(engine) = engine.borrow_mut().as_mut() {
            render_and_track(engine, canvas_view);
        }
        return;
    };

    let engine_for_cb = Rc::clone(engine);
    let holder: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let holder_for_cb = Rc::clone(&holder);
    let cb = Closure::wrap(Box::new(move |_ts: f64| {
        raf_pending.set(false);
        if let Some(engine) = engine_for_cb.borrow_mut().as_mut() {
            render_and_track(engine, canvas_view);
        }
        holder_for_cb.borrow_mut().take();
    }) as Box<dyn FnMut(f64)>);

    if window
        .request_animation_frame(cb.as_ref().unchecked_ref())
        .is_ok()
    {
        *holder.borrow_mut() = Some(cb);
    } else {
        raf_pending.set(false);
        if let Some(engine) = engine.borrow_mut().as_mut() {
            render_and_track(engine, canvas_view);
        }
    }
}

#[cfg(feature = "hydrate")]
fn mount_dials_into_panels() {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let (Some(host), Some(mount)) = (
        document.get_element_by_id("left-dials-host"),
        document.get_element_by_id("left-dials-mount"),
    ) {
        let _ = mount.append_child(&host);
    }
    if let (Some(host), Some(mount)) = (
        document.get_element_by_id("right-dials-host"),
        document.get_element_by_id("right-dials-mount"),
    ) {
        let _ = mount.append_child(&host);
    }
}

/// Canvas host component.
///
/// On hydration, this mounts `canvas::engine::Engine`, synchronizes board
/// objects from websocket state, and renders on updates.
#[component]
pub fn CanvasHost() -> impl IntoView {
    let _auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let canvas_view = expect_context::<RwSignal<CanvasViewState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
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
    let object_text_style_ref = NodeRef::<leptos::html::Div>::new();
    let _object_text_style_drag_active = RwSignal::new(false);
    #[cfg(feature = "hydrate")]
    let object_rotate_drag_state = RwSignal::new(None::<SelectionRotationDragState>);
    #[cfg(feature = "hydrate")]
    let object_zoom_drag_state = RwSignal::new(None::<SelectionScaleDragState>);
    #[cfg(feature = "hydrate")]
    let object_color_drag_state = RwSignal::new(None::<SelectionColorDragState>);
    #[cfg(feature = "hydrate")]
    let object_border_drag_state = RwSignal::new(None::<SelectionBorderDragState>);
    #[cfg(feature = "hydrate")]
    let object_text_style_drag_state = RwSignal::new(None::<SelectionTextStyleDragState>);
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
    let last_center_override_seq = RwSignal::new(0_u64);
    #[cfg(feature = "hydrate")]
    let last_scene_sync_key = RwSignal::new((None::<String>, 0_u64, None::<String>, 0_i64));
    #[cfg(feature = "hydrate")]
    let render_raf_pending = RwSignal::new(false);
    #[cfg(feature = "hydrate")]
    let animation_tick = Rc::new(RefCell::new(None::<Interval>));
    #[cfg(feature = "hydrate")]
    let preview_cursor = RwSignal::new(None::<CanvasPoint>);
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
            let prior_view = canvas_view.get_untracked();
            if prior_view.viewport_width > 0.0 && prior_view.viewport_height > 0.0 {
                instance.core.camera.pan_x = prior_view.pan_x;
                instance.core.camera.pan_y = prior_view.pan_y;
                instance.core.camera.zoom = prior_view.zoom.clamp(0.1, 10.0);
                instance.set_view_rotation_deg(prior_view.view_rotation_deg);
            } else {
                center_world_origin(&mut instance);
            }
            sync_canvas_view_state(&instance, canvas_view, None);
            send_cursor_presence_if_needed(
                &instance,
                board,
                _auth,
                sender,
                last_presence_sent_ms,
                last_presence_sent,
                None,
                true,
            );
            render_and_track(&mut instance, canvas_view);
            *engine.borrow_mut() = Some(instance);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move || {
            // Re-run on route/mode transitions so dial hosts are re-parented
            // even when board navigation changes mount timing.
            let _ = _ui.get().view_mode;
            let _ = board.get().board_id.clone();
            mount_dials_into_panels();

            // Also retry on the next tick to handle cases where panel mounts
            // are inserted slightly after this effect first runs.
            Timeout::new(0, move || {
                mount_dials_into_panels();
            })
            .forget();
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_bootstrap = canvas_ref.clone();
        Effect::new(move || {
            let state = board.get();
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
                    board,
                    _auth,
                    sender,
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
        let animation_tick = Rc::clone(&animation_tick);
        let engine = Rc::clone(&engine);
        let render_raf_pending = render_raf_pending;
        Effect::new(move || {
            let ui_state = _ui.get();
            let maybe_clip = board.with(|state| resolve_active_clip(state, &ui_state));
            if !ui_state.animation_playing {
                animation_tick.borrow_mut().take();
                return;
            }
            let Some((_clip_id, clip)) = maybe_clip else {
                animation_tick.borrow_mut().take();
                _ui.update(|u| u.animation_playing = false);
                return;
            };
            if clip.duration_ms <= 0.0 {
                animation_tick.borrow_mut().take();
                _ui.update(|u| u.animation_playing = false);
                return;
            }
            if animation_tick.borrow().is_some() {
                return;
            }

            let engine_for_tick = Rc::clone(&engine);
            let board_for_tick = board;
            let ui_for_tick = _ui;
            let canvas_view_for_tick = canvas_view;
            let render_raf_pending_for_tick = render_raf_pending;
            let tick = Interval::new(33, move || {
                let maybe_clip = board_for_tick.with(|state| resolve_active_clip(state, &ui_for_tick.get_untracked()));
                ui_for_tick.update(|u| {
                    if !u.animation_playing {
                        return;
                    }
                    let Some((_clip_id, clip)) = maybe_clip.as_ref() else {
                        u.animation_playing = false;
                        return;
                    };
                    let mut next = u.animation_playhead_ms + 33.0;
                    if next > clip.duration_ms {
                        if clip.looped {
                            next = if clip.duration_ms > 0.0 {
                                next % clip.duration_ms
                            } else {
                                0.0
                            };
                        } else {
                            next = clip.duration_ms;
                            u.animation_playing = false;
                        }
                    }
                    u.animation_playhead_ms = next.max(0.0);
                });
                request_render(&engine_for_tick, canvas_view_for_tick, render_raf_pending_for_tick);
            });
            *animation_tick.borrow_mut() = Some(tick);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_sync = canvas_ref.clone();
        let render_raf_pending = render_raf_pending;
        Effect::new(move || {
            let ui_state = _ui.get();
            let Some(scene_key) = board.with(|state| {
                if state.join_streaming {
                    None
                } else {
                    let active_clip = resolve_active_clip(state, &ui_state).map(|(id, _)| id);
                    let playhead = ui_state.animation_playhead_ms.round() as i64;
                    Some((state.board_id.clone(), state.scene_rev, active_clip, playhead))
                }
            }) else {
                return;
            };
            if last_scene_sync_key.get_untracked() == scene_key {
                return;
            }

            let snapshot = board.with(|state| {
                let board_id = state.board_id.clone();
                let mut scene_objects = HashMap::with_capacity(state.objects.len());
                for (id, obj) in &state.objects {
                    scene_objects.insert(id.clone(), state.drag_objects.get(id).unwrap_or(obj).clone());
                }
                if let Some((_id, clip)) = resolve_active_clip(state, &ui_state) {
                    scene_objects =
                        project_clip_scene(&scene_objects, board_id.as_deref(), &clip, ui_state.animation_playhead_ms);
                }

                let mut snapshot = Vec::with_capacity(scene_objects.len());
                for obj in scene_objects.values() {
                    if let Some(mapped) = to_canvas_object(obj, board_id.as_deref()) {
                        snapshot.push(mapped);
                    }
                }
                snapshot
            });

            if let Some(engine) = engine.borrow_mut().as_mut() {
                engine.load_snapshot(snapshot);
                sync_viewport(engine, &canvas_ref_sync);
                sync_canvas_view_state(engine, canvas_view, None);
            }
            if let Some(engine) = engine.borrow().as_ref() {
                send_cursor_presence_if_needed(
                    engine,
                    board,
                    _auth,
                    sender,
                    last_presence_sent_ms,
                    last_presence_sent,
                    None,
                    true,
                );
            }
            request_render(&engine, canvas_view, render_raf_pending);
            last_scene_sync_key.set(scene_key);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        Effect::new(move || {
            let tool = map_tool(_ui.get().active_tool);
            if let Some(engine) = engine.borrow_mut().as_mut() {
                engine.set_tool(tool);
            }
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_home = canvas_ref.clone();
        let render_raf_pending = render_raf_pending;
        Effect::new(move || {
            let seq = _ui.get().home_viewport_seq;
            if seq == last_home_viewport_seq.get_untracked() {
                return;
            }
            if let Some(engine) = engine.borrow_mut().as_mut() {
                sync_viewport(engine, &canvas_ref_home);
                center_world_origin(engine);
                sync_canvas_view_state(engine, canvas_view, None);
                send_cursor_presence_if_needed(
                    engine,
                    board,
                    _auth,
                    sender,
                    last_presence_sent_ms,
                    last_presence_sent,
                    None,
                    true,
                );
            }
            request_render(&engine, canvas_view, render_raf_pending);
            last_home_viewport_seq.set(seq);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_zoom = canvas_ref.clone();
        let render_raf_pending = render_raf_pending;
        Effect::new(move || {
            let ui_state = _ui.get();
            let seq = ui_state.zoom_override_seq;
            if seq == 0 || seq == last_zoom_override_seq.get_untracked() {
                return;
            }
            let target_zoom = ui_state.zoom_override;
            let mut should_render = false;
            if let Some(engine) = engine.borrow_mut().as_mut() {
                if let Some(zoom) = target_zoom {
                    sync_viewport(engine, &canvas_ref_zoom);
                    let center_screen = viewport_center_screen(engine);
                    let center_world = engine
                        .camera()
                        .screen_to_world(center_screen, center_screen);
                    let rotation = engine.view_rotation_deg();
                    set_camera_view(engine, center_world.x, center_world.y, zoom, rotation);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    should_render = true;
                }
            }
            if should_render {
                request_render(&engine, canvas_view, render_raf_pending);
            }
            last_zoom_override_seq.set(seq);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_center = canvas_ref.clone();
        let render_raf_pending = render_raf_pending;
        Effect::new(move || {
            let ui_state = _ui.get();
            let seq = ui_state.view_center_override_seq;
            if seq == 0 || seq == last_center_override_seq.get_untracked() {
                return;
            }
            let target_center = ui_state.view_center_override;
            let mut should_render = false;
            if let Some(engine) = engine.borrow_mut().as_mut()
                && let Some((center_x, center_y)) = target_center
            {
                sync_viewport(engine, &canvas_ref_center);
                let zoom = engine.camera().zoom;
                let rotation = engine.view_rotation_deg();
                set_camera_view(engine, center_x, center_y, zoom, rotation);
                sync_canvas_view_state(engine, canvas_view, None);
                send_cursor_presence_if_needed(
                    engine,
                    board,
                    _auth,
                    sender,
                    last_presence_sent_ms,
                    last_presence_sent,
                    None,
                    true,
                );
                should_render = true;
            }
            if should_render {
                request_render(&engine, canvas_view, render_raf_pending);
            }
            last_center_override_seq.set(seq);
        });
    }

    #[cfg(feature = "hydrate")]
    {
        let engine = Rc::clone(&engine);
        let canvas_ref_follow = canvas_ref.clone();
        let render_raf_pending = render_raf_pending;
        Effect::new(move || {
            let Some((target_client, target)) = board.with(|state| {
                let target_client = state
                    .jump_to_client_id
                    .clone()
                    .or_else(|| state.follow_client_id.clone())?;
                let target = state.presence.get(&target_client).cloned()?;
                Some((target_client, target))
            }) else {
                return;
            };
            let Some(center) = target.camera_center else {
                return;
            };
            let Some(zoom) = target.camera_zoom else {
                return;
            };
            let rotation = target.camera_rotation.unwrap_or(0.0);
            let mut should_render = false;
            if let Some(engine) = engine.borrow_mut().as_mut() {
                sync_viewport(engine, &canvas_ref_follow);
                set_camera_view(engine, center.x, center.y, zoom, rotation);
                sync_canvas_view_state(engine, canvas_view, None);
                should_render = true;
                if board.get_untracked().jump_to_client_id.as_deref() == Some(target_client.as_str()) {
                    board.update(|b| b.jump_to_client_id = None);
                }
            }
            if should_render {
                request_render(&engine, canvas_view, render_raf_pending);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                let point = pointer_point(&ev);
                if let Some((kind, width, height, props)) = placement_shape(_ui.get().active_tool) {
                    {
                        let engine_ref = engine.borrow();
                        if let Some(engine) = engine_ref.as_ref() {
                            place_shape_at_cursor(point, kind, width, height, props, engine, board, sender);
                        }
                    }
                    if let Some(engine) = engine.borrow_mut().as_mut() {
                        _ui.update(|u| u.active_tool = ToolType::Select);
                        preview_cursor.set(None);
                        render_and_track(engine, canvas_view);
                    }
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_down(point, button, modifiers);
                    process_actions(actions, engine, board, sender);
                    sync_canvas_view_state(engine, canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        Some(point),
                        false,
                    );
                    render_and_track(engine, canvas_view);
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
                if board.get().selection.is_empty() {
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
                if board.get().follow_client_id.is_some() {
                    if let Some(engine) = engine.borrow().as_ref() {
                        sync_canvas_view_state(engine, canvas_view, Some(point));
                        send_cursor_presence_if_needed(
                            engine,
                            board,
                            _auth,
                            sender,
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
                            board,
                            _auth,
                            sender,
                            last_presence_sent_ms,
                            last_presence_sent,
                            Some(point),
                            false,
                        );
                        sync_canvas_view_state(engine, canvas_view, Some(point));
                    }
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_move(point, modifiers);
                    process_actions(actions, engine, board, sender);
                    sync_canvas_view_state(engine, canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        Some(point),
                        false,
                    );
                    send_object_drag_if_needed(engine, board, sender, last_drag_sent_ms);
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    let active_transform = active_transform_object_ids(engine);
                    sync_viewport(engine, &canvas_ref);
                    let point = pointer_point(&ev);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_up(point, button, modifiers);
                    process_actions(actions, engine, board, sender);
                    sync_selection_from_engine(engine, board);
                    sync_canvas_view_state(engine, canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        Some(point),
                        false,
                    );
                    last_drag_sent_ms.set(0.0);
                    send_object_drag_end(active_transform, board, sender);
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    let point = wheel_point(&ev);
                    let delta = WheelDelta { dx: ev.delta_x(), dy: ev.delta_y() };
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_wheel(point, delta, modifiers);
                    process_actions(actions, engine, board, sender);
                    sync_canvas_view_state(engine, canvas_view, Some(point));
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    render_and_track(engine, canvas_view);
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
            let canvas_ref = canvas_ref.clone();
            let engine = Rc::clone(&engine);
            move |ev: leptos::ev::PointerEvent| {
                preview_cursor.set(None);
                if let Some(canvas) = canvas_ref.get() {
                    let _ = canvas.release_pointer_capture(ev.pointer_id());
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    let active_transform = active_transform_object_ids(engine);
                    sync_viewport(engine, &canvas_ref);
                    let point = pointer_point(&ev);
                    let button = map_button(ev.button());
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_pointer_up(point, button, modifiers);
                    process_actions(actions, engine, board, sender);
                    sync_selection_from_engine(engine, board);
                    sync_canvas_view_state(engine, canvas_view, None);
                    last_drag_sent_ms.set(0.0);
                    send_object_drag_end(active_transform, board, sender);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    render_and_track(engine, canvas_view);
                }
                send_cursor_clear(board, sender);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if key == "Escape" && placement_shape(_ui.get().active_tool).is_some() {
                    ev.prevent_default();
                    _ui.update(|u| u.active_tool = ToolType::Select);
                    preview_cursor.set(None);
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    let active_transform = active_transform_object_ids(engine);
                    sync_viewport(engine, &canvas_ref);
                    if should_prevent_default_key(&key) {
                        ev.prevent_default();
                    }
                    let key_for_engine = key.clone();
                    let modifiers = map_modifiers(ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key());
                    let actions = engine.on_key_down(CanvasKey(key_for_engine), modifiers);
                    process_actions(actions, engine, board, sender);
                    sync_selection_from_engine(engine, board);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    if key == "Escape" {
                        send_object_drag_end(active_transform, board, sender);
                    }
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
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
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    render_and_track(engine, canvas_view);
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
                if !_compass_drag_active.get_untracked() || board.get().follow_client_id.is_some() {
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
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    render_and_track(engine, canvas_view);
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
                        board,
                        _auth,
                        sender,
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(0.0);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(90.0);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(180.0);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    engine.set_view_rotation_deg(270.0);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    render_and_track(engine, canvas_view);
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
                if board.get().follow_client_id.is_some() {
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
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    render_and_track(engine, canvas_view);
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
                if !_zoom_drag_active.get_untracked() || board.get().follow_client_id.is_some() {
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
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        false,
                    );
                    render_and_track(engine, canvas_view);
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
                        board,
                        _auth,
                        sender,
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
                if board.get().follow_client_id.is_some() {
                    return;
                }
                if let Some(engine) = engine.borrow_mut().as_mut() {
                    sync_viewport(engine, &canvas_ref);
                    zoom_view_preserving_center(engine, 1.0);
                    sync_canvas_view_state(engine, canvas_view, None);
                    send_cursor_presence_if_needed(
                        engine,
                        board,
                        _auth,
                        sender,
                        last_presence_sent_ms,
                        last_presence_sent,
                        None,
                        true,
                    );
                    render_and_track(engine, canvas_view);
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
                if !has_selection(board) {
                    return;
                }
                let Some(dial) = object_color_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_color_seed(board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_color_drag_state.set(Some(drag_state));
                _object_color_drag_active.set(true);
                apply_selection_color_shift(board, object_color_drag_state, color_shift_from_dial_angle(angle));
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
                apply_selection_color_shift(board, object_color_drag_state, color_shift_from_dial_angle(angle));
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
                commit_selection_color_updates(board, sender, object_color_drag_state);
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
                apply_group_base_color_target(board, sender, input.value());
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
                if !has_selection(board) {
                    return;
                }
                let Some(dial) = object_border_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_border_seed(board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_border_drag_state.set(Some(drag_state));
                _object_border_drag_active.set(true);
                apply_selection_border_width(board, object_border_drag_state, border_width_from_dial_angle(angle));
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
                apply_selection_border_width(board, object_border_drag_state, border_width_from_dial_angle(angle));
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
                commit_selection_border_updates(board, sender, object_border_drag_state);
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
                apply_group_border_color_target(board, sender, input.value());
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
            move |_ev: leptos::ev::MouseEvent| apply_group_background_defaults_target(board, sender)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_border_reset = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_border_defaults_target(board, sender)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_text_style_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let object_text_style_ref = object_text_style_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if pointer_event_hits_control(&ev, ".canvas-color-dial__picker, .canvas-color-dial__readout") {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                if !has_selection(board) {
                    return;
                }
                let Some(dial) = object_text_style_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_text_style_seed(board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_text_style_drag_state.set(Some(drag_state));
                _object_text_style_drag_active.set(true);
                apply_selection_font_size(board, object_text_style_drag_state, font_size_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };
    let on_object_text_style_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let object_text_style_ref = object_text_style_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if !_object_text_style_drag_active.get_untracked() {
                    return;
                }
                let Some(dial) = object_text_style_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                apply_selection_font_size(board, object_text_style_drag_state, font_size_from_dial_angle(angle));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };
    let on_object_text_style_pointer_up = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::PointerEvent| {
                _object_text_style_drag_active.set(false);
                commit_selection_text_style_updates(board, sender, object_text_style_drag_state);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };
    let on_object_text_style_readout_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    };
    let on_object_text_style_input = {
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
                apply_group_text_color_target(board, sender, input.value());
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::Event| {}
        }
    };
    let on_object_text_style_reset = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| apply_group_text_style_defaults_target(board, sender)
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
                if pointer_event_hits_control(&ev, ".canvas-zoom-wheel__readout, .canvas-zoom-wheel__reset") {
                    return;
                }
                ev.prevent_default();
                ev.stop_propagation();
                if !has_selection(board) {
                    return;
                }
                let Some(dial) = object_zoom_ref.get() else {
                    return;
                };
                let Some(angle) = zoom_angle_from_pointer(&ev, &dial) else {
                    return;
                };
                let Some(drag_state) = selection_scale_seed(board) else {
                    return;
                };
                let _ = dial.set_pointer_capture(ev.pointer_id());
                object_zoom_drag_state.set(Some(drag_state));
                _object_zoom_drag_active.set(true);
                apply_selection_scale_drag(board, object_zoom_drag_state, zoom_from_dial_angle(angle));
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
                apply_selection_scale_drag(board, object_zoom_drag_state, zoom_from_dial_angle(angle));
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
                commit_selection_scale_updates(board, sender, object_zoom_drag_state);
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

    let object_color_base = move || representative_base_color_hex(&board.get());
    let object_color_shift = move || representative_lightness_shift(&board.get());
    let object_color_knob_style = move || {
        let angle = dial_angle_from_color_shift(object_color_shift());
        format!("transform: rotate({angle:.2}deg);")
    };
    let object_border_color = move || representative_border_color_hex(&board.get());
    let object_border_width = move || representative_border_width(&board.get());
    let object_border_knob_style = move || {
        let angle = dial_angle_from_border_width(object_border_width());
        format!("transform: rotate({angle:.2}deg);")
    };
    let object_text_color = move || representative_text_color_hex(&board.get());
    let object_text_size = move || representative_font_size(&board.get());
    let object_text_knob_style = move || {
        let angle = dial_angle_from_font_size(object_text_size());
        format!("transform: rotate({angle:.2}deg);")
    };

    let object_zoom_scale = move || representative_scale_factor(&board.get());
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

                let start_rotations = selected_object_rotations(board);
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
                apply_selection_rotation_drag(
                    board,
                    object_rotate_drag_state,
                    angle,
                    ev.shift_key(),
                    apply_compass_drag_snapping,
                    signed_angle_delta_deg,
                    normalize_degrees_360,
                );
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
                commit_selection_rotation_updates(board, sender, object_rotate_drag_state, angular_delta_deg);
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
            move |_ev: leptos::ev::MouseEvent| {
                apply_group_rotation_target(board, sender, 0.0, signed_angle_delta_deg, normalize_degrees_360)
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_snap_e = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| {
                apply_group_rotation_target(board, sender, 90.0, signed_angle_delta_deg, normalize_degrees_360)
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_snap_s = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| {
                apply_group_rotation_target(board, sender, 180.0, signed_angle_delta_deg, normalize_degrees_360)
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_snap_w = {
        #[cfg(feature = "hydrate")]
        {
            move |_ev: leptos::ev::MouseEvent| {
                apply_group_rotation_target(board, sender, 270.0, signed_angle_delta_deg, normalize_degrees_360)
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };
    let on_object_rotate_center_click = move |_ev: leptos::ev::MouseEvent| {};

    let object_rotation_angle_deg = move || representative_rotation_deg(&board.get());
    let object_rotation_knob_style = move || {
        let angle = object_rotation_angle_deg();
        format!("transform: rotate({angle:.2}deg);")
    };
    let has_selected_objects = move || !board.get().selection.is_empty();

    let compass_angle_deg = move || normalize_degrees_360(canvas_view.get().view_rotation_deg);
    let compass_knob_style = move || {
        let angle = compass_angle_deg();
        format!("transform: rotate({angle:.2}deg);")
    };
    let zoom_percent = move || canvas_view.get().zoom * 100.0;
    let zoom_knob_style = move || {
        #[cfg(feature = "hydrate")]
        {
            let angle = dial_angle_from_zoom(canvas_view.get().zoom);
            return format!("transform: rotate({angle:.2}deg);");
        }
        #[cfg(not(feature = "hydrate"))]
        {
            "transform: rotate(0deg);".to_owned()
        }
    };

    let canvas_world_overlay_style = move || {
        let view = canvas_view.get();
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
            let view = canvas_view.get();
            let pan_x = view.pan_x;
            let pan_y = view.pan_y;
            let zoom = view.zoom;
            return board
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
            if placement_shape(_ui.get().active_tool).is_none() {
                return None::<(String, String)>;
            }
            let point = preview_cursor
                .get()
                .unwrap_or_else(|| CanvasPoint::new(40.0, 40.0));
            let style = format!("left: {:.2}px; top: {:.2}px;", point.x, point.y);
            Some(("canvas-placement-ghost".to_owned(), style))
        }
        #[cfg(not(feature = "hydrate"))]
        {
            None::<(String, String)>
        }
    };

    view! {
        <>
            {view! {
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
            }
            .into_any()}
            {move || {
                preview_ghost().map(|(class_name, style)| {
                    view! { <div class=class_name style=style></div> }
                })
            }}
            {view! {
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
                </div>
            }
            .into_any()}
            {view! {
                <div id="left-dials-host" class="left-panel__dials-mount">
                    <ZoomDial
                        class="canvas-object-zoom"
                        disabled_class="canvas-object-zoom--disabled"
                        title="Drag to scale selected object(s); top is neutral"
                        readout_title="Click to reset selected object scale to 100%"
                        reset_title="Reset selected object scale to 100%"
                        knob_class="canvas-object-zoom__knob"
                        node_ref=object_zoom_ref
                        disabled=Signal::derive(move || !has_selected_objects())
                        readout=Signal::derive(move || format!("{:.0}%", object_zoom_scale() * 100.0))
                        knob_style=Signal::derive(object_zoom_knob_style)
                        on_pointer_down=on_object_zoom_pointer_down
                        on_pointer_move=on_object_zoom_pointer_move
                        on_pointer_up=on_object_zoom_pointer_up
                        on_readout_pointer_down=on_object_zoom_readout_pointer_down
                        on_readout_click=move |_ev| apply_group_scale_target(board, sender, 1.0)
                        on_readout_dblclick=move |_ev| apply_group_scale_target(board, sender, 1.0)
                        on_reset_click=move |_ev| apply_group_scale_target(board, sender, 1.0)
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
                    <ColorDial
                        class="canvas-object-text-style"
                        disabled_class="canvas-object-text-style--disabled"
                        title="Drag to set selected text size; center picks text color"
                        swatch_title="Selected text color"
                        reset_title="Reset text style to defaults"
                        center_label=Signal::derive(move || format_text_size_label(object_text_size()))
                        knob_class="canvas-object-text-style__knob"
                        node_ref=object_text_style_ref
                        disabled=Signal::derive(move || !has_selected_objects())
                        knob_style=Signal::derive(object_text_knob_style)
                        color_value=Signal::derive(object_text_color)
                        on_pointer_down=on_object_text_style_pointer_down
                        on_pointer_move=on_object_text_style_pointer_move
                        on_pointer_up=on_object_text_style_pointer_up
                        on_center_pointer_down=on_object_text_style_readout_pointer_down
                        on_color_input=on_object_text_style_input
                        on_reset_click=on_object_text_style_reset
                    />
                </div>
            }
            .into_any()}
            {view! {
                <div
                    id="right-dials-host"
                    class="right-panel__dials-mount"
                    style=move || if _ui.get().view_mode == ViewMode::Trace { "display:none;" } else { "" }
                >
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
                        reset_title="Reset zoom to 100%"
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
                        on_readout_dblclick=on_zoom_reset.clone()
                        on_reset_click=on_zoom_reset
                    />
                </div>
            }
            .into_any()}
        </>
    }
}

#[cfg(feature = "hydrate")]
fn sync_selection_from_engine(engine: &Engine, board: RwSignal<BoardState>) {
    let selected = engine
        .selections()
        .into_iter()
        .map(|id| id.to_string())
        .collect::<std::collections::HashSet<_>>();
    board.update(|b| {
        if b.selection == selected {
            return;
        }
        b.selection = selected
            .into_iter()
            .filter(|id| b.objects.contains_key(id))
            .collect();
    });
}

#[cfg(feature = "hydrate")]
fn active_transform_object_ids(engine: &Engine) -> Vec<String> {
    match engine.core.input.clone() {
        CanvasInputState::DraggingObject { ids, .. } => ids.into_iter().map(|id| id.to_string()).collect(),
        CanvasInputState::ResizingObject { id, .. }
        | CanvasInputState::RotatingObject { id, .. }
        | CanvasInputState::DraggingEdgeEndpoint { id, .. } => vec![id.to_string()],
        _ => Vec::new(),
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
        trace: None,
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

    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let ids: Vec<uuid::Uuid> = match engine.core.input.clone() {
        CanvasInputState::DraggingObject { duplicated: true, .. } => return,
        CanvasInputState::DraggingObject { ids, .. } => ids,
        CanvasInputState::ResizingObject { id, .. }
        | CanvasInputState::RotatingObject { id, .. }
        | CanvasInputState::DraggingEdgeEndpoint { id, .. } => vec![id],
        _ => return,
    };

    let mut sent = false;
    for object_id in ids {
        let Some(obj) = engine.object(&object_id) else {
            continue;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id.clone()),
            from: None,
            syscall: "object:drag".to_owned(),
            status: FrameStatus::Request,
            trace: None,
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
        sent = sender.get_untracked().send(&frame) || sent;
    }
    if sent {
        last_sent_ms.set(now);
    }
}

#[cfg(feature = "hydrate")]
fn send_object_drag_end(ids: Vec<String>, board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    if ids.is_empty() {
        return;
    }
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    for id in ids {
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id.clone()),
            from: None,
            syscall: "object:drag:end".to_owned(),
            status: FrameStatus::Request,
            trace: None,
            data: serde_json::json!({ "id": id }),
        };
        let _ = sender.get_untracked().send(&frame);
    }
}

fn remote_cursor_style(x: f64, y: f64, color: &str) -> String {
    format!("transform: translate({x:.2}px, {y:.2}px); --cursor-color: {color};")
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
    let local_object = BoardObject {
        id: id.clone(),
        board_id: board_id.clone(),
        kind: kind.to_owned(),
        x,
        y,
        width: Some(width),
        height: Some(height),
        rotation: 0.0,
        z_index: i32::try_from(board.get_untracked().objects.len()).unwrap_or(i32::MAX),
        props: props.clone(),
        created_by: Some("local".to_owned()),
        version: 1,
        group_id: None,
    };

    board.update(|b| {
        b.objects.insert(id.clone(), local_object);
        b.selection.clear();
        b.selection.insert(id.clone());
        b.bump_scene_rev();
    });

    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id.clone()),
        from: None,
        syscall: "object:create".to_owned(),
        status: FrameStatus::Request,
        trace: None,
        data: serde_json::json!({
            "id": id,
            "kind": kind,
            "x": x,
            "y": y,
            "width": width,
            "height": height,
            "rotation": 0,
            "props": props,
            "group_id": null,
        }),
    };
    let _ = sender.get_untracked().send(&frame);
}

#[cfg(feature = "hydrate")]
fn to_canvas_object(obj: &crate::net::types::BoardObject, active_board_id: Option<&str>) -> Option<CanvasObject> {
    let id = parse_or_stable_uuid(&obj.id);
    let board_id = active_board_id
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .or_else(|| uuid::Uuid::parse_str(&obj.board_id).ok())
        .unwrap_or(uuid::Uuid::nil());
    let created_by = obj
        .created_by
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    let kind = match obj.kind.as_str() {
        "rectangle" | "sticky_note" => CanvasKind::Rect,
        "text" => CanvasKind::Text,
        "frame" => CanvasKind::Frame,
        "ellipse" => CanvasKind::Ellipse,
        "diamond" => CanvasKind::Diamond,
        "star" => CanvasKind::Star,
        "line" => CanvasKind::Line,
        "arrow" => CanvasKind::Arrow,
        "svg" => CanvasKind::Svg,
        _ => CanvasKind::Rect,
    };

    let width = obj.width.unwrap_or(120.0).max(1.0);
    let height = obj.height.unwrap_or(80.0).max(1.0);
    let props = obj.props.clone();

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
        group_id: obj
            .group_id
            .as_deref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok()),
    })
}

#[cfg(feature = "hydrate")]
fn parse_or_stable_uuid(value: &str) -> uuid::Uuid {
    if let Ok(parsed) = uuid::Uuid::parse_str(value) {
        return parsed;
    }
    // Animation event streams may use human-readable IDs (for example "ball1").
    // Derive a stable pseudo-UUID so transient playback objects still render.
    let mut h1 = DefaultHasher::new();
    "collabboard-animation-id-a".hash(&mut h1);
    value.hash(&mut h1);
    let hi = h1.finish();
    let mut h2 = DefaultHasher::new();
    "collabboard-animation-id-b".hash(&mut h2);
    value.hash(&mut h2);
    let lo = h2.finish();
    let bytes = [
        (hi >> 56) as u8,
        (hi >> 48) as u8,
        (hi >> 40) as u8,
        (hi >> 32) as u8,
        (hi >> 24) as u8,
        (hi >> 16) as u8,
        (hi >> 8) as u8,
        hi as u8,
        (lo >> 56) as u8,
        (lo >> 48) as u8,
        (lo >> 40) as u8,
        (lo >> 32) as u8,
        (lo >> 24) as u8,
        (lo >> 16) as u8,
        (lo >> 8) as u8,
        lo as u8,
    ];
    uuid::Uuid::from_bytes(bytes)
}

#[cfg(feature = "hydrate")]
fn process_actions(
    actions: Vec<Action>,
    engine: &mut Engine,
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
) {
    const LOCAL_OBJECT_PATCH_LIMIT: usize = 500;

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
                        b.bump_scene_rev();
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
                    trace: None,
                    data: serde_json::json!({
                        "id": obj.id.to_string(),
                        "kind": canvas_kind_to_wire(obj.kind),
                        "x": obj.x,
                        "y": obj.y,
                        "width": obj.width,
                        "height": obj.height,
                        "rotation": obj.rotation,
                        "props": obj.props,
                        "group_id": obj.group_id.map(|id| id.to_string()),
                    }),
                };
                let _ = sender.get_untracked().send(&frame);
            }
            Action::ObjectUpdated { id, fields } => {
                let Some(board_id) = board.get_untracked().board_id else {
                    continue;
                };
                let geometry_changed = fields.width.is_some() || fields.height.is_some();
                let can_patch_local = board.get_untracked().objects.len() <= LOCAL_OBJECT_PATCH_LIMIT;
                if can_patch_local
                    && let Some(obj) = engine.object(&id)
                    && let Some(mut local) = to_wire_object(obj, &board_id)
                {
                    if geometry_changed {
                        reset_wire_object_scale_baseline(&mut local);
                    }
                    board.update(|b| {
                        b.objects.insert(local.id.clone(), local);
                        b.bump_scene_rev();
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
                if let Some(group_id) = fields.group_id {
                    data.insert("group_id".to_owned(), serde_json::json!(group_id.map(|id| id.to_string())));
                }
                if let Some(props) = fields.props {
                    data.insert("props".to_owned(), props);
                }
                if geometry_changed && let Some(obj) = engine.object(&id) {
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
                    trace: None,
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
                    b.bump_scene_rev();
                });

                let frame = Frame {
                    id: uuid::Uuid::new_v4().to_string(),
                    parent_id: None,
                    ts: 0,
                    board_id: Some(board_id),
                    from: None,
                    syscall: "object:delete".to_owned(),
                    status: FrameStatus::Request,
                    trace: None,
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
        CanvasKind::Line => "line",
        CanvasKind::Arrow => "arrow",
        CanvasKind::Svg => "svg",
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
        group_id: obj.group_id.map(|id| id.to_string()),
    })
}
