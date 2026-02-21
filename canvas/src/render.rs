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

use crate::camera::{Camera, Point};
use crate::consts::{FRAC_PI_5, HANDLE_RADIUS_PX, STAR_INNER_RATIO};
use crate::doc::{BoardObject, DocStore, ObjectKind, Props};
use crate::hit;
use crate::input::UiState;

/// Arrowhead length in world units.
const ARROW_SIZE: f64 = 10.0;

/// Arrowhead half-angle in radians (~30°).
const ARROW_ANGLE: f64 = PI / 6.0;

/// Selection dash segment length in screen pixels.
const SELECTION_DASH_PX: f64 = 4.0;
/// Small visual marker for an endpoint attached to another shape.
const ATTACHED_ANCHOR_RADIUS_WORLD: f64 = 3.0;

/// Draw the full scene: objects and selection UI.
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
    let viewport_center = Point::new(viewport_w * 0.5, viewport_h * 0.5);

    // Layer 1: clear and set up transforms.
    ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0)?;
    ctx.clear_rect(0.0, 0.0, viewport_w, viewport_h);
    ctx.translate(viewport_center.x, viewport_center.y)?;
    ctx.rotate(camera.view_rotation_deg.to_radians())?;
    ctx.translate(-viewport_center.x, -viewport_center.y)?;
    ctx.translate(camera.pan_x, camera.pan_y)?;
    ctx.scale(camera.zoom, camera.zoom)?;

    // Layer 2: objects in z-order (bottom first).
    for obj in doc.sorted_objects() {
        draw_object(ctx, obj, doc)?;
    }

    // Layer 3: selection UI.
    let selected = ui.selected_ids.iter().copied().collect::<Vec<_>>();
    let show_handles = selected.len() == 1;
    for sel_id in selected {
        if let Some(obj) = doc.get(&sel_id) {
            draw_selection(ctx, obj, doc, camera.zoom, show_handles)?;
        }
    }

    if let Some(m) = ui.marquee {
        draw_marquee(ctx, m, camera.zoom)?;
    }

    Ok(())
}

// =============================================================
// Object dispatch
// =============================================================

fn draw_object(ctx: &CanvasRenderingContext2d, obj: &BoardObject, doc: &DocStore) -> Result<(), JsValue> {
    let props = Props::new(&obj.props);

    match obj.kind {
        ObjectKind::Rect => draw_rect(ctx, obj, &props),
        ObjectKind::Text => draw_text_object(ctx, obj, &props),
        ObjectKind::Frame => draw_frame(ctx, obj, &props),
        ObjectKind::Ellipse => draw_ellipse(ctx, obj, &props),
        ObjectKind::Diamond => draw_diamond(ctx, obj, &props),
        ObjectKind::Star => draw_star(ctx, obj, &props),
        ObjectKind::Youtube => draw_youtube(ctx, obj, &props),
        ObjectKind::Line | ObjectKind::Arrow => {
            draw_edge(ctx, obj, doc, &props, obj.kind == ObjectKind::Arrow);
            Ok(())
        }
    }
}

fn draw_text_object(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    ctx.save();
    translate_and_rotate(ctx, obj)?;
    draw_text(ctx, obj, props)?;
    ctx.restore();
    Ok(())
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

fn draw_frame(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    ctx.save();
    translate_and_rotate(ctx, obj)?;

    let title_h = (obj.height * 0.14).clamp(18.0, 28.0);
    let x = -obj.width * 0.5;
    let y = -obj.height * 0.5;

    // Body fill is very subtle so children remain visible.
    ctx.set_fill_style_str("rgba(60, 64, 70, 0.06)");
    ctx.fill_rect(x, y, obj.width, obj.height);

    // Border.
    apply_stroke_style(ctx, props);
    ctx.stroke_rect(x, y, obj.width, obj.height);

    // Header band.
    ctx.set_fill_style_str("rgba(31, 26, 23, 0.16)");
    ctx.fill_rect(x, y, obj.width, title_h);

    // Title
    let title = obj
        .props
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Frame");
    ctx.set_fill_style_str("#1F1A17");
    ctx.set_text_align("left");
    ctx.set_text_baseline("middle");
    let font_size = (title_h * 0.45).clamp(10.0, 14.0);
    ctx.set_font(&format!("{font_size:.0}px sans-serif"));
    ctx.fill_text(title, x + 8.0, y + (title_h * 0.5))?;

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

fn draw_youtube(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    if obj.width <= 0.0 || obj.height <= 0.0 {
        return Ok(());
    }

    let shell = "#4a4037";
    let bezel = "#2f2823";
    let screen = "#111";
    let glow = "#1d1d1d";

    ctx.save();
    translate_and_rotate(ctx, obj)?;

    let hw = obj.width * 0.5;
    let hh = obj.height * 0.5;

    // TV shell
    ctx.set_fill_style_str(shell);
    ctx.fill_rect(-hw, -hh, obj.width, obj.height);
    ctx.set_stroke_style_str(props.stroke());
    ctx.set_line_width(props.stroke_width().max(1.0));
    ctx.stroke_rect(-hw, -hh, obj.width, obj.height);

    // Screen bezel
    let bezel_pad_x = obj.width * 0.08;
    let bezel_pad_y = obj.height * 0.14;
    let bezel_w = obj.width - (bezel_pad_x * 2.0);
    let bezel_h = obj.height - (bezel_pad_y * 2.0) - (obj.height * 0.10);
    ctx.set_fill_style_str(bezel);
    ctx.fill_rect(-bezel_w * 0.5, -hh + bezel_pad_y, bezel_w, bezel_h);

    // Screen
    let screen_pad = obj.width.min(obj.height) * 0.04;
    let screen_w = bezel_w - (screen_pad * 2.0);
    let screen_h = bezel_h - (screen_pad * 2.0);
    let screen_x = -screen_w * 0.5;
    let screen_y = -hh + bezel_pad_y + screen_pad;
    ctx.set_fill_style_str(screen);
    ctx.fill_rect(screen_x, screen_y, screen_w, screen_h);
    ctx.set_fill_style_str(glow);
    ctx.fill_rect(screen_x + 2.0, screen_y + 2.0, screen_w - 4.0, screen_h - 4.0);

    // Play button
    let play_r = obj.width.min(obj.height) * 0.12;
    let play_cx = 0.0;
    let play_cy = screen_y + (screen_h * 0.5);
    ctx.set_fill_style_str("#fff");
    ctx.begin_path();
    ctx.arc(play_cx, play_cy, play_r, 0.0, 2.0 * PI)?;
    ctx.fill();
    ctx.set_fill_style_str("#d12b2b");
    ctx.begin_path();
    ctx.move_to(play_cx - (play_r * 0.30), play_cy - (play_r * 0.45));
    ctx.line_to(play_cx + (play_r * 0.50), play_cy);
    ctx.line_to(play_cx - (play_r * 0.30), play_cy + (play_r * 0.45));
    ctx.close_path();
    ctx.fill();

    // Antennas
    ctx.set_stroke_style_str(props.stroke());
    ctx.set_line_width(2.0);
    let ant_base_y = -hh + 2.0;
    ctx.begin_path();
    ctx.move_to(-obj.width * 0.20, ant_base_y);
    ctx.line_to(-obj.width * 0.34, -hh - (obj.height * 0.18));
    ctx.move_to(obj.width * 0.20, ant_base_y);
    ctx.line_to(obj.width * 0.34, -hh - (obj.height * 0.18));
    ctx.stroke();

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
    let a_attached = endpoint_is_attached(obj, "a");
    let b_attached = endpoint_is_attached(obj, "b");

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

    // Attachment marker in normal mode so snapped endpoints are visible.
    ctx.set_fill_style_str("#fff");
    for (pt, attached) in [(a, a_attached), (b, b_attached)] {
        if !attached {
            continue;
        }
        ctx.begin_path();
        if ctx
            .arc(pt.x, pt.y, ATTACHED_ANCHOR_RADIUS_WORLD, 0.0, 2.0 * PI)
            .is_ok()
        {
            ctx.fill();
        }
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

    // Font size: explicit prop wins; otherwise derive from object height.
    let font_size = props
        .font_size()
        .unwrap_or_else(|| (obj.height / 6.0).clamp(12.0, 24.0))
        .clamp(8.0, 96.0);

    ctx.save();
    ctx.set_fill_style_str(props.text_color());
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let font_str = format!("{font_size}px sans-serif");
    ctx.set_font(&font_str);

    // Vertical layout: head near top, wrapped body centered, foot near bottom.
    let max_w = (obj.width - 12.0).max(1.0);
    if !head.is_empty() {
        let y = -hh + font_size;
        let head_fit = fit_text_with_ellipsis(ctx, head, max_w);
        ctx.fill_text(&head_fit, 0.0, y)?;
    }
    if !text.is_empty() {
        let line_height = (font_size * 1.25).max(12.0);
        let max_lines = ((obj.height / line_height).floor() as usize).max(1);
        let mut lines = wrap_text_lines(ctx, text, max_w);
        if lines.len() > max_lines {
            lines.truncate(max_lines);
            if let Some(last) = lines.last_mut() {
                *last = fit_text_with_ellipsis(ctx, last, max_w);
            }
        }
        let total_height = line_height * (lines.len().saturating_sub(1) as f64);
        let start_y = -total_height * 0.5;
        for (idx, line) in lines.iter().enumerate() {
            let y = start_y + (idx as f64 * line_height);
            ctx.fill_text(line, 0.0, y)?;
        }
    }
    if !foot.is_empty() {
        let y = hh - font_size;
        let foot_fit = fit_text_with_ellipsis(ctx, foot, max_w);
        ctx.fill_text(&foot_fit, 0.0, y)?;
    }

    ctx.restore();
    Ok(())
}

fn wrap_text_lines(ctx: &CanvasRenderingContext2d, text: &str, max_w: f64) -> Vec<String> {
    let mut out = Vec::new();
    for raw_line in text.lines() {
        let words: Vec<&str> = raw_line.split_whitespace().collect();
        if words.is_empty() {
            out.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in words {
            if current.is_empty() {
                if measured_text_width(ctx, word) <= max_w {
                    current.push_str(word);
                } else {
                    let mut chunks = break_long_word(ctx, word, max_w);
                    if let Some(last) = chunks.pop() {
                        out.extend(chunks);
                        current = last;
                    }
                }
                continue;
            }

            let candidate = format!("{current} {word}");
            if measured_text_width(ctx, &candidate) <= max_w {
                current = candidate;
            } else {
                out.push(std::mem::take(&mut current));
                if measured_text_width(ctx, word) <= max_w {
                    current = word.to_owned();
                } else {
                    let mut chunks = break_long_word(ctx, word, max_w);
                    if let Some(last) = chunks.pop() {
                        out.extend(chunks);
                        current = last;
                    } else {
                        current.clear();
                    }
                }
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn break_long_word(ctx: &CanvasRenderingContext2d, word: &str, max_w: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        let mut candidate = current.clone();
        candidate.push(ch);
        if !current.is_empty() && measured_text_width(ctx, &candidate) > max_w {
            lines.push(current);
            current = ch.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn fit_text_with_ellipsis(ctx: &CanvasRenderingContext2d, text: &str, max_w: f64) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if measured_text_width(ctx, trimmed) <= max_w {
        return trimmed.to_owned();
    }

    let ellipsis = "...";
    let mut chars: Vec<char> = trimmed.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate = format!("{}{}", chars.iter().collect::<String>().trim_end(), ellipsis);
        if measured_text_width(ctx, &candidate) <= max_w {
            return candidate;
        }
    }
    ellipsis.to_owned()
}

fn measured_text_width(ctx: &CanvasRenderingContext2d, text: &str) -> f64 {
    match ctx.measure_text(text) {
        Ok(metrics) => metrics.width(),
        Err(_) => f64::INFINITY,
    }
}

// =============================================================
// Selection UI
// =============================================================

fn draw_selection(
    ctx: &CanvasRenderingContext2d,
    obj: &BoardObject,
    doc: &DocStore,
    zoom: f64,
    show_handles: bool,
) -> Result<(), JsValue> {
    match obj.kind {
        ObjectKind::Line | ObjectKind::Arrow => draw_edge_selection(ctx, obj, doc, zoom, show_handles),
        _ => draw_node_selection(ctx, obj, zoom, show_handles),
    }
}

fn draw_node_selection(
    ctx: &CanvasRenderingContext2d,
    obj: &BoardObject,
    zoom: f64,
    show_handles: bool,
) -> Result<(), JsValue> {
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

    if !show_handles {
        return Ok(());
    }

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

fn draw_edge_selection(
    ctx: &CanvasRenderingContext2d,
    obj: &BoardObject,
    doc: &DocStore,
    zoom: f64,
    show_handles: bool,
) -> Result<(), JsValue> {
    let Some(a) = hit::edge_endpoint_a_resolved(obj, doc) else {
        return Ok(());
    };
    let Some(b) = hit::edge_endpoint_b_resolved(obj, doc) else {
        return Ok(());
    };

    let handle_radius = if show_handles {
        HANDLE_RADIUS_PX / zoom
    } else {
        (HANDLE_RADIUS_PX * 0.6) / zoom
    };

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

fn draw_marquee(
    ctx: &CanvasRenderingContext2d,
    marquee: crate::input::SelectionRect,
    zoom: f64,
) -> Result<(), JsValue> {
    ctx.save();
    let dash_world = SELECTION_DASH_PX / zoom;
    let dash_array = js_sys::Array::new();
    dash_array.push(&dash_world.into());
    dash_array.push(&dash_world.into());
    ctx.set_line_dash(&dash_array)?;
    ctx.set_stroke_style_str("#1E90FF");
    ctx.set_fill_style_str("rgba(30, 144, 255, 0.12)");
    ctx.set_line_width(1.0 / zoom);
    ctx.fill_rect(marquee.x, marquee.y, marquee.width, marquee.height);
    ctx.stroke_rect(marquee.x, marquee.y, marquee.width, marquee.height);
    ctx.set_line_dash(&js_sys::Array::new())?;
    ctx.restore();
    Ok(())
}

fn endpoint_is_attached(obj: &BoardObject, key: &str) -> bool {
    obj.props
        .get(key)
        .and_then(|v| v.get("type"))
        .and_then(serde_json::Value::as_str)
        == Some("attached")
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
