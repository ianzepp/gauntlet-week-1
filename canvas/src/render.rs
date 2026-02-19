//! Rendering: draws the full canvas scene to a 2D context.
//!
//! This module is the only place that touches [`web_sys::CanvasRenderingContext2d`].
//! It receives read-only views of document state and camera state and produces
//! pixels — it does not mutate any application state.
//!
//! All fallible `Canvas2D` calls propagate errors via `Result<(), JsValue>`.
//! The top-level caller ([`crate::engine::Engine::render`]) handles the result.

use std::f64::consts::PI;

use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

use crate::camera::Camera;
use crate::consts::{FRAC_PI_5, HANDLE_RADIUS_PX, STAR_INNER_RATIO};
use crate::doc::{BoardObject, DocStore, ObjectKind, Props};
use crate::hit;
use crate::input::UiState;

/// Grid spacing in world units at zoom = 1.0.
const GRID_SPACING: f64 = 20.0;

/// Minimum zoom level below which the grid is hidden entirely.
const GRID_MIN_ZOOM: f64 = 0.2;

/// Arrowhead length in world units.
const ARROW_SIZE: f64 = 10.0;

/// Arrowhead half-angle in radians (~30°).
const ARROW_ANGLE: f64 = PI / 6.0;

/// Selection dash segment length in screen pixels.
const SELECTION_DASH_PX: f64 = 4.0;

/// Draw the full scene: grid, objects, selection UI.
///
/// `viewport_w` and `viewport_h` are in CSS pixels. `dpr` is the device pixel ratio.
///
/// # Errors
///
/// Returns `Err` if any `Canvas2D` call fails (e.g. invalid context state).
pub fn draw(
    ctx: &CanvasRenderingContext2d,
    doc: &DocStore,
    camera: &Camera,
    ui: &UiState,
    viewport_w: f64,
    viewport_h: f64,
    dpr: f64,
) -> Result<(), JsValue> {
    // Layer 1: clear and set up transforms.
    ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0)?;
    ctx.clear_rect(0.0, 0.0, viewport_w, viewport_h);
    ctx.translate(camera.pan_x, camera.pan_y)?;
    ctx.scale(camera.zoom, camera.zoom)?;

    // Layer 2: grid.
    draw_grid(ctx, camera, viewport_w, viewport_h)?;

    // Layer 3: objects in z-order (bottom first).
    for obj in doc.sorted_objects() {
        draw_object(ctx, obj, doc)?;
    }

    // Layer 4: selection UI.
    if let Some(sel_id) = ui.selected_id {
        if let Some(obj) = doc.get(&sel_id) {
            draw_selection(ctx, obj, doc, camera.zoom)?;
        }
    }

    Ok(())
}

// =============================================================
// Grid
// =============================================================

fn draw_grid(ctx: &CanvasRenderingContext2d, camera: &Camera, viewport_w: f64, viewport_h: f64) -> Result<(), JsValue> {
    if camera.zoom < GRID_MIN_ZOOM {
        return Ok(());
    }

    // Adapt spacing: double it when zoom is low.
    let spacing = if camera.zoom < 0.5 {
        GRID_SPACING * 2.0
    } else {
        GRID_SPACING
    };

    // Compute visible world bounds.
    let world_left = -camera.pan_x / camera.zoom;
    let world_top = -camera.pan_y / camera.zoom;
    let world_right = (viewport_w - camera.pan_x) / camera.zoom;
    let world_bottom = (viewport_h - camera.pan_y) / camera.zoom;

    // Snap to grid.
    let start_x = (world_left / spacing).floor() * spacing;
    let start_y = (world_top / spacing).floor() * spacing;

    // Dot radius in world units (1 CSS pixel).
    let dot_radius = 1.0 / camera.zoom;

    ctx.save();
    ctx.set_fill_style_str("#ccc");

    let mut x = start_x;
    while x <= world_right {
        let mut y = start_y;
        while y <= world_bottom {
            ctx.begin_path();
            ctx.arc(x, y, dot_radius, 0.0, 2.0 * PI)?;
            ctx.fill();
            y += spacing;
        }
        x += spacing;
    }

    ctx.restore();
    Ok(())
}

// =============================================================
// Object dispatch
// =============================================================

fn draw_object(ctx: &CanvasRenderingContext2d, obj: &BoardObject, doc: &DocStore) -> Result<(), JsValue> {
    let props = Props::new(&obj.props);

    match obj.kind {
        ObjectKind::Rect => draw_rect(ctx, obj, &props),
        ObjectKind::Ellipse => draw_ellipse(ctx, obj, &props),
        ObjectKind::Diamond => draw_diamond(ctx, obj, &props),
        ObjectKind::Star => draw_star(ctx, obj, &props),
        ObjectKind::Line | ObjectKind::Arrow => {
            draw_edge(ctx, obj, doc, &props, obj.kind == ObjectKind::Arrow);
            Ok(())
        }
    }
}

// =============================================================
// Shape renderers
// =============================================================

fn draw_rect(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    ctx.save();
    translate_and_rotate(ctx, obj)?;

    ctx.set_fill_style_str(props.fill());
    ctx.fill_rect(-obj.width / 2.0, -obj.height / 2.0, obj.width, obj.height);

    apply_stroke_style(ctx, props);
    ctx.stroke_rect(-obj.width / 2.0, -obj.height / 2.0, obj.width, obj.height);

    draw_text(ctx, obj, props)?;
    ctx.restore();
    Ok(())
}

fn draw_ellipse(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    if obj.width <= 0.0 || obj.height <= 0.0 {
        return Ok(());
    }
    ctx.save();
    translate_and_rotate(ctx, obj)?;

    ctx.begin_path();
    ctx.ellipse(0.0, 0.0, obj.width / 2.0, obj.height / 2.0, 0.0, 0.0, 2.0 * PI)?;

    ctx.set_fill_style_str(props.fill());
    ctx.fill();

    apply_stroke_style(ctx, props);
    ctx.stroke();

    draw_text(ctx, obj, props)?;
    ctx.restore();
    Ok(())
}

fn draw_diamond(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    ctx.save();
    translate_and_rotate(ctx, obj)?;

    let hw = obj.width / 2.0;
    let hh = obj.height / 2.0;

    ctx.begin_path();
    ctx.move_to(0.0, -hh); // top
    ctx.line_to(hw, 0.0); // right
    ctx.line_to(0.0, hh); // bottom
    ctx.line_to(-hw, 0.0); // left
    ctx.close_path();

    ctx.set_fill_style_str(props.fill());
    ctx.fill();

    apply_stroke_style(ctx, props);
    ctx.stroke();

    draw_text(ctx, obj, props)?;
    ctx.restore();
    Ok(())
}

#[allow(clippy::similar_names)]
fn draw_star(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    if obj.width <= 0.0 || obj.height <= 0.0 {
        return Ok(());
    }
    ctx.save();
    translate_and_rotate(ctx, obj)?;

    let rx_outer = obj.width / 2.0;
    let ry_outer = obj.height / 2.0;
    let rx_inner = rx_outer * STAR_INNER_RATIO;
    let ry_inner = ry_outer * STAR_INNER_RATIO;

    let offset = std::f64::consts::FRAC_PI_2;

    ctx.begin_path();
    for i in 0..10 {
        let angle = FRAC_PI_5.mul_add(f64::from(i), -offset);
        let (rx, ry) = if i % 2 == 0 {
            (rx_outer, ry_outer)
        } else {
            (rx_inner, ry_inner)
        };
        let px = rx * angle.cos();
        let py = ry * angle.sin();
        if i == 0 {
            ctx.move_to(px, py);
        } else {
            ctx.line_to(px, py);
        }
    }
    ctx.close_path();

    ctx.set_fill_style_str(props.fill());
    ctx.fill();

    apply_stroke_style(ctx, props);
    ctx.stroke();

    draw_text(ctx, obj, props)?;
    ctx.restore();
    Ok(())
}

// =============================================================
// Edge renderers
// =============================================================

fn draw_edge(ctx: &CanvasRenderingContext2d, obj: &BoardObject, doc: &DocStore, props: &Props<'_>, arrowhead: bool) {
    let Some(a) = hit::edge_endpoint_a_resolved(obj, doc) else {
        return;
    };
    let Some(b) = hit::edge_endpoint_b_resolved(obj, doc) else {
        return;
    };

    ctx.save();
    apply_stroke_style(ctx, props);

    ctx.begin_path();
    ctx.move_to(a.x, a.y);
    ctx.line_to(b.x, b.y);
    ctx.stroke();

    if arrowhead {
        let angle = (b.y - a.y).atan2(b.x - a.x);
        draw_arrowhead(ctx, b.x, b.y, angle);
    }

    ctx.restore();
}

fn draw_arrowhead(ctx: &CanvasRenderingContext2d, tip_x: f64, tip_y: f64, angle: f64) {
    let x1 = tip_x - ARROW_SIZE * (angle - ARROW_ANGLE).cos();
    let y1 = tip_y - ARROW_SIZE * (angle - ARROW_ANGLE).sin();
    let x2 = tip_x - ARROW_SIZE * (angle + ARROW_ANGLE).cos();
    let y2 = tip_y - ARROW_SIZE * (angle + ARROW_ANGLE).sin();

    ctx.begin_path();
    ctx.move_to(tip_x, tip_y);
    ctx.line_to(x1, y1);
    ctx.line_to(x2, y2);
    ctx.close_path();
    ctx.fill();
}

// =============================================================
// Text
// =============================================================

#[allow(clippy::similar_names)]
fn draw_text(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    let head = props.head();
    let text = props.text();
    let foot = props.foot();

    // Nothing to draw.
    if head.is_empty() && text.is_empty() && foot.is_empty() {
        return Ok(());
    }

    let hh = obj.height / 2.0;

    // Font size: clamp between 12 and 24, scaling with shape height.
    let font_size = (obj.height / 6.0).clamp(12.0, 24.0);

    ctx.save();
    ctx.set_fill_style_str("#1F1A17");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let font_str = format!("{font_size}px sans-serif");
    ctx.set_font(&font_str);

    // Vertical layout: head near top, text at center, foot near bottom.
    let max_w = obj.width.max(1.0);
    if !head.is_empty() {
        let y = -hh + font_size;
        ctx.fill_text_with_max_width(head, 0.0, y, max_w)?;
    }
    if !text.is_empty() {
        ctx.fill_text_with_max_width(text, 0.0, 0.0, max_w)?;
    }
    if !foot.is_empty() {
        let y = hh - font_size;
        ctx.fill_text_with_max_width(foot, 0.0, y, max_w)?;
    }

    ctx.restore();
    Ok(())
}

// =============================================================
// Selection UI
// =============================================================

fn draw_selection(ctx: &CanvasRenderingContext2d, obj: &BoardObject, doc: &DocStore, zoom: f64) -> Result<(), JsValue> {
    match obj.kind {
        ObjectKind::Line | ObjectKind::Arrow => draw_edge_selection(ctx, obj, doc, zoom),
        _ => draw_node_selection(ctx, obj, zoom),
    }
}

fn draw_node_selection(ctx: &CanvasRenderingContext2d, obj: &BoardObject, zoom: f64) -> Result<(), JsValue> {
    ctx.save();

    // Dashed bounding box (rotated with the object).
    translate_and_rotate(ctx, obj)?;

    let hw = obj.width / 2.0;
    let hh = obj.height / 2.0;
    let dash_world = SELECTION_DASH_PX / zoom;

    ctx.set_stroke_style_str("#1E90FF");
    ctx.set_line_width(1.0 / zoom);
    let dash_array = js_sys::Array::new();
    dash_array.push(&dash_world.into());
    dash_array.push(&dash_world.into());
    ctx.set_line_dash(&dash_array)?;

    ctx.stroke_rect(-hw, -hh, obj.width, obj.height);
    ctx.set_line_dash(&js_sys::Array::new())?;

    ctx.restore();

    // Resize handles (drawn in world coordinates, not rotated context).
    let handle_size_world = HANDLE_RADIUS_PX / zoom;
    let handles = hit::resize_handle_positions(obj.x, obj.y, obj.width, obj.height, obj.rotation);

    ctx.save();
    ctx.set_fill_style_str("#fff");
    ctx.set_stroke_style_str("#1E90FF");
    ctx.set_line_width(1.0 / zoom);

    for pos in &handles {
        ctx.fill_rect(
            pos.x - handle_size_world,
            pos.y - handle_size_world,
            handle_size_world * 2.0,
            handle_size_world * 2.0,
        );
        ctx.stroke_rect(
            pos.x - handle_size_world,
            pos.y - handle_size_world,
            handle_size_world * 2.0,
            handle_size_world * 2.0,
        );
    }

    // Rotate handle.
    let rh = hit::rotate_handle_position(obj.x, obj.y, obj.width, obj.height, obj.rotation, zoom);
    let n_handle = handles[0]; // N handle

    // Connecting line from N handle to rotate handle.
    ctx.set_stroke_style_str("#1E90FF");
    ctx.begin_path();
    ctx.move_to(n_handle.x, n_handle.y);
    ctx.line_to(rh.x, rh.y);
    ctx.stroke();

    // Rotate handle circle.
    ctx.begin_path();
    ctx.arc(rh.x, rh.y, handle_size_world, 0.0, 2.0 * PI)?;
    ctx.set_fill_style_str("#fff");
    ctx.fill();
    ctx.stroke();

    ctx.restore();
    Ok(())
}

fn draw_edge_selection(ctx: &CanvasRenderingContext2d, obj: &BoardObject, doc: &DocStore, zoom: f64) -> Result<(), JsValue> {
    let Some(a) = hit::edge_endpoint_a_resolved(obj, doc) else {
        return Ok(());
    };
    let Some(b) = hit::edge_endpoint_b_resolved(obj, doc) else {
        return Ok(());
    };

    let handle_radius = HANDLE_RADIUS_PX / zoom;

    ctx.save();
    ctx.set_fill_style_str("#fff");
    ctx.set_stroke_style_str("#1E90FF");
    ctx.set_line_width(1.0 / zoom);

    for pt in &[a, b] {
        ctx.begin_path();
        ctx.arc(pt.x, pt.y, handle_radius, 0.0, 2.0 * PI)?;
        ctx.fill();
        ctx.stroke();
    }

    ctx.restore();
    Ok(())
}

// =============================================================
// Helpers
// =============================================================

/// Translate to the object's center and rotate by its rotation angle.
fn translate_and_rotate(ctx: &CanvasRenderingContext2d, obj: &BoardObject) -> Result<(), JsValue> {
    let cx = obj.x + obj.width / 2.0;
    let cy = obj.y + obj.height / 2.0;
    ctx.translate(cx, cy)?;
    ctx.rotate(obj.rotation.to_radians())?;
    Ok(())
}

/// Apply stroke style and line width from props.
fn apply_stroke_style(ctx: &CanvasRenderingContext2d, props: &Props<'_>) {
    ctx.set_stroke_style_str(props.stroke());
    ctx.set_line_width(props.stroke_width());
}
