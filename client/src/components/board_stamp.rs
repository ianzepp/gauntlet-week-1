//! Top-right canvas minimap overlay.
//!
//! SYSTEM CONTEXT
//! ==============
//! Renders a scaled view of board world objects and highlights current viewport
//! bounds using camera telemetry from `CanvasViewState`.

use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use crate::state::canvas_view::CanvasViewState;
#[cfg(feature = "hydrate")]
use crate::state::ui::UiState;

#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast;

/// Board minimap overlay.
#[component]
pub fn BoardStamp() -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let board = expect_context::<RwSignal<BoardState>>();
    #[cfg(feature = "hydrate")]
    let canvas_view = expect_context::<RwSignal<CanvasViewState>>();
    #[cfg(feature = "hydrate")]
    let ui = expect_context::<RwSignal<UiState>>();
    let minimap_ref = NodeRef::<leptos::html::Canvas>::new();
    let dragging = RwSignal::new(false);

    #[cfg(feature = "hydrate")]
    {
        let minimap_ref = minimap_ref.clone();
        Effect::new(move || {
            let objects = board.get().objects.values().cloned().collect::<Vec<_>>();
            let view = canvas_view.get();
            let Some(canvas) = minimap_ref.get() else {
                return;
            };
            draw_minimap(&canvas, &objects, &view);
        });
    }

    let on_minimap_pointer_down = {
        #[cfg(feature = "hydrate")]
        {
            let minimap_ref = minimap_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                ev.prevent_default();
                ev.stop_propagation();
                let Some(canvas) = minimap_ref.get() else {
                    return;
                };
                let objects = board.get_untracked().objects.values().cloned().collect::<Vec<_>>();
                let view = canvas_view.get_untracked();
                let Some((center_x, center_y)) = minimap_pointer_to_world_center(&canvas, &objects, &view, &ev) else {
                    return;
                };
                ui.update(|u| {
                    u.view_center_override = Some((center_x, center_y));
                    u.view_center_override_seq = u.view_center_override_seq.saturating_add(1);
                });
                dragging.set(true);
                let _ = canvas.set_pointer_capture(ev.pointer_id());
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_minimap_pointer_move = {
        #[cfg(feature = "hydrate")]
        {
            let minimap_ref = minimap_ref.clone();
            move |ev: leptos::ev::PointerEvent| {
                if !dragging.get_untracked() {
                    return;
                }
                let Some(canvas) = minimap_ref.get() else {
                    return;
                };
                let objects = board.get_untracked().objects.values().cloned().collect::<Vec<_>>();
                let view = canvas_view.get_untracked();
                let Some((center_x, center_y)) = minimap_pointer_to_world_center(&canvas, &objects, &view, &ev) else {
                    return;
                };
                let damped_x = view.camera_center_world.x + ((center_x - view.camera_center_world.x) * 0.72);
                let damped_y = view.camera_center_world.y + ((center_y - view.camera_center_world.y) * 0.72);
                ui.update(|u| {
                    u.view_center_override = Some((damped_x, damped_y));
                    u.view_center_override_seq = u.view_center_override_seq.saturating_add(1);
                });
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::PointerEvent| {}
        }
    };

    let on_minimap_pointer_up = move |_ev: leptos::ev::PointerEvent| {
        dragging.set(false);
    };

    view! {
        <canvas
            class="board-stamp__minimap"
            node_ref=minimap_ref
            aria-label="Board minimap"
            on:pointerdown=on_minimap_pointer_down
            on:pointermove=on_minimap_pointer_move
            on:pointerup=on_minimap_pointer_up
            on:pointercancel=on_minimap_pointer_up
            on:pointerleave=on_minimap_pointer_up
        ></canvas>
    }
}

#[cfg(feature = "hydrate")]
fn draw_minimap(
    canvas: &web_sys::HtmlCanvasElement,
    objects: &[crate::net::types::BoardObject],
    view: &CanvasViewState,
) {
    let width_css = f64::from(canvas.client_width().max(1));
    let height_css = f64::from(canvas.client_height().max(1));
    if canvas.width() != width_css.round() as u32 || canvas.height() != height_css.round() as u32 {
        canvas.set_width(width_css.round() as u32);
        canvas.set_height(height_css.round() as u32);
    }

    let Some(ctx_value) = canvas.get_context("2d").ok().flatten() else {
        return;
    };
    let Some(ctx) = ctx_value.dyn_into::<web_sys::CanvasRenderingContext2d>().ok() else {
        return;
    };

    ctx.set_fill_style_str("#f6f1e7");
    ctx.fill_rect(0.0, 0.0, width_css, height_css);
    ctx.set_stroke_style_str("#cec3b4");
    ctx.stroke_rect(0.5, 0.5, (width_css - 1.0).max(0.0), (height_css - 1.0).max(0.0));

    let viewport_w_world = (view.viewport_width / view.zoom.max(0.001)).max(10.0);
    let viewport_h_world = (view.viewport_height / view.zoom.max(0.001)).max(10.0);
    let viewport_x = view.camera_center_world.x - (viewport_w_world * 0.5);
    let viewport_y = view.camera_center_world.y - (viewport_h_world * 0.5);

    let transform = minimap_transform(objects, view, width_css, height_css);
    let world_to_canvas = |x: f64, y: f64| -> (f64, f64) {
        (
            transform.offset_x + ((x - transform.min_x) * transform.scale),
            transform.offset_y + ((y - transform.min_y) * transform.scale),
        )
    };

    for obj in objects {
        let (x, y, w, h) = object_world_rect(obj);
        let (cx, cy) = world_to_canvas(x, y);
        let cw = (w * transform.scale).max(1.0);
        let ch = (h * transform.scale).max(1.0);
        let fill = object_fill_color(obj);
        ctx.set_fill_style_str(fill.as_str());
        ctx.fill_rect(cx, cy, cw, ch);
        ctx.set_stroke_style_str("#3d3428");
        ctx.stroke_rect(cx, cy, cw, ch);
    }

    let (vx, vy) = world_to_canvas(viewport_x, viewport_y);
    let vw = (viewport_w_world * transform.scale).max(1.0);
    let vh = (viewport_h_world * transform.scale).max(1.0);
    ctx.set_stroke_style_str("#8b4049");
    ctx.set_line_width(1.5);
    ctx.stroke_rect(vx, vy, vw, vh);
}

#[cfg(feature = "hydrate")]
#[derive(Clone, Copy)]
struct MinimapTransform {
    min_x: f64,
    min_y: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

#[cfg(feature = "hydrate")]
fn minimap_transform(
    objects: &[crate::net::types::BoardObject],
    view: &CanvasViewState,
    width_css: f64,
    height_css: f64,
) -> MinimapTransform {
    let viewport_w_world = (view.viewport_width / view.zoom.max(0.001)).max(10.0);
    let viewport_h_world = (view.viewport_height / view.zoom.max(0.001)).max(10.0);
    let viewport_x = view.camera_center_world.x - (viewport_w_world * 0.5);
    let viewport_y = view.camera_center_world.y - (viewport_h_world * 0.5);

    let mut min_x = viewport_x;
    let mut min_y = viewport_y;
    let mut max_x = viewport_x + viewport_w_world;
    let mut max_y = viewport_y + viewport_h_world;

    for obj in objects {
        let (x, y, w, h) = object_world_rect(obj);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let world_w = (max_x - min_x).max(1.0);
    let world_h = (max_y - min_y).max(1.0);
    let pad = 8.0;
    let inner_w = (width_css - (pad * 2.0)).max(1.0);
    let inner_h = (height_css - (pad * 2.0)).max(1.0);
    let scale = (inner_w / world_w).min(inner_h / world_h).max(0.0001);
    let offset_x = pad + ((inner_w - (world_w * scale)) * 0.5);
    let offset_y = pad + ((inner_h - (world_h * scale)) * 0.5);
    MinimapTransform { min_x, min_y, scale, offset_x, offset_y }
}

#[cfg(feature = "hydrate")]
fn minimap_pointer_to_world_center(
    canvas: &web_sys::HtmlCanvasElement,
    objects: &[crate::net::types::BoardObject],
    view: &CanvasViewState,
    ev: &leptos::ev::PointerEvent,
) -> Option<(f64, f64)> {
    let width_css = f64::from(canvas.client_width().max(1));
    let height_css = f64::from(canvas.client_height().max(1));
    let transform = minimap_transform(objects, view, width_css, height_css);
    let x_px = f64::from(ev.offset_x());
    let y_px = f64::from(ev.offset_y());
    let world_x = ((x_px - transform.offset_x) / transform.scale) + transform.min_x;
    let world_y = ((y_px - transform.offset_y) / transform.scale) + transform.min_y;
    if !world_x.is_finite() || !world_y.is_finite() {
        return None;
    }
    Some((world_x, world_y))
}

#[cfg(feature = "hydrate")]
fn object_world_rect(obj: &crate::net::types::BoardObject) -> (f64, f64, f64, f64) {
    let w = obj.width.unwrap_or(120.0).max(2.0);
    let h = obj.height.unwrap_or(80.0).max(2.0);
    (obj.x, obj.y, w, h)
}

#[cfg(feature = "hydrate")]
fn object_fill_color(obj: &crate::net::types::BoardObject) -> String {
    obj.props
        .get("backgroundColor")
        .or_else(|| obj.props.get("fill"))
        .and_then(|v| v.as_str())
        .unwrap_or("#b8c5b0")
        .to_owned()
}
