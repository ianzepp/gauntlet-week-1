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
use web_sys::{CanvasRenderingContext2d, Path2d};

use crate::camera::{Camera, Point};
use crate::consts::{FRAC_PI_5, HANDLE_RADIUS_PX, STAR_INNER_RATIO};
use crate::doc::{BoardObject, DocStore, ObjectKind, Props, WorldBounds};
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
    let viewport_bounds = viewport_world_bounds(camera, viewport_w, viewport_h, viewport_center);
    let visible = doc.sorted_objects_in_bounds(viewport_bounds);

    // Layer 1: clear and set up transforms.
    ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0)?;
    ctx.clear_rect(0.0, 0.0, viewport_w, viewport_h);
    ctx.translate(viewport_center.x, viewport_center.y)?;
    ctx.rotate(camera.view_rotation_deg.to_radians())?;
    ctx.translate(-viewport_center.x, -viewport_center.y)?;
    ctx.translate(camera.pan_x, camera.pan_y)?;
    ctx.scale(camera.zoom, camera.zoom)?;

    // Layer 2: non-selected objects in z-order.
    for obj in &visible {
        if ui.selected_ids.contains(&obj.id) {
            continue;
        }
        draw_object(ctx, obj, doc)?;
    }

    // Layer 3: selected objects in z-order.
    // Multi-select is naturally supported because selected_ids is a set.
    for obj in &visible {
        if !ui.selected_ids.contains(&obj.id) {
            continue;
        }
        draw_object(ctx, obj, doc)?;
    }

    // Layer 4: selection UI overlays.
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

fn viewport_world_bounds(camera: &Camera, viewport_w: f64, viewport_h: f64, viewport_center: Point) -> WorldBounds {
    let corners = [
        Point::new(0.0, 0.0),
        Point::new(viewport_w, 0.0),
        Point::new(0.0, viewport_h),
        Point::new(viewport_w, viewport_h),
    ];

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for corner in corners {
        let world = camera.screen_to_world(corner, viewport_center);
        min_x = min_x.min(world.x);
        min_y = min_y.min(world.y);
        max_x = max_x.max(world.x);
        max_y = max_y.max(world.y);
    }

    // Small margin avoids visible pop-in at the viewport edge.
    let margin = camera.screen_dist_to_world(64.0);
    WorldBounds { min_x: min_x - margin, min_y: min_y - margin, max_x: max_x + margin, max_y: max_y + margin }
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
        ObjectKind::Svg => draw_svg_placeholder(ctx, obj, &props),
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

fn draw_svg_placeholder(ctx: &CanvasRenderingContext2d, obj: &BoardObject, props: &Props<'_>) -> Result<(), JsValue> {
    if draw_inline_svg_paths(ctx, obj).is_ok() {
        return Ok(());
    }

    ctx.save();
    translate_and_rotate(ctx, obj)?;

    ctx.set_fill_style_str(props.fill());
    ctx.fill_rect(-obj.width / 2.0, -obj.height / 2.0, obj.width, obj.height);
    apply_stroke_style(ctx, props);
    ctx.stroke_rect(-obj.width / 2.0, -obj.height / 2.0, obj.width, obj.height);

    let label = obj
        .props
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("SVG");
    ctx.set_fill_style_str(props.text_color());
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let font_size = (obj.height / 5.0).clamp(10.0, 20.0);
    ctx.set_font(&format!("{font_size:.0}px sans-serif"));
    ctx.fill_text(label, 0.0, 0.0)?;

    ctx.restore();
    Ok(())
}

fn draw_inline_svg_paths(ctx: &CanvasRenderingContext2d, obj: &BoardObject) -> Result<(), JsValue> {
    let Some(svg) = obj.props.get("svg").and_then(serde_json::Value::as_str) else {
        return Err(JsValue::from_str("missing svg prop"));
    };
    let shapes = parse_svg_shapes(svg);
    if shapes.is_empty() {
        return Err(JsValue::from_str("no renderable elements"));
    }

    let (vb_min_x, vb_min_y, vb_w, vb_h) = svg_view_box(svg).unwrap_or((0.0, 0.0, obj.width, obj.height));
    let vb_w = vb_w.max(1.0);
    let vb_h = vb_h.max(1.0);
    let sx = obj.width / vb_w;
    let sy = obj.height / vb_h;
    let stroke_scale = ((sx.abs() + sy.abs()) * 0.5).max(0.000_001);

    ctx.save();
    translate_and_rotate(ctx, obj)?;
    ctx.translate(-obj.width * 0.5, -obj.height * 0.5)?;
    ctx.scale(sx, sy)?;
    ctx.translate(-vb_min_x, -vb_min_y)?;

    for shape in &shapes {
        render_svg_shape(ctx, shape, stroke_scale)?;
    }

    ctx.restore();
    Ok(())
}

/// Render a single `SvgShape` to the canvas context.
fn render_svg_shape(ctx: &CanvasRenderingContext2d, shape: &SvgShape, stroke_scale: f64) -> Result<(), JsValue> {
    match &shape.geometry {
        SvgGeometry::Path(d) => {
            let path = Path2d::new_with_path_string(d)?;
            svg_fill_stroke(
                ctx,
                &path,
                shape.fill.as_deref(),
                shape.stroke.as_deref(),
                shape.stroke_width,
                stroke_scale,
            );
        }
        SvgGeometry::Rect { x, y, width, height, rx, ry } => {
            let path = svg_rect_path(*x, *y, *width, *height, *rx, *ry)?;
            svg_fill_stroke(
                ctx,
                &path,
                shape.fill.as_deref(),
                shape.stroke.as_deref(),
                shape.stroke_width,
                stroke_scale,
            );
        }
        SvgGeometry::Circle { cx, cy, r } => {
            let path = svg_circle_path(*cx, *cy, *r)?;
            svg_fill_stroke(
                ctx,
                &path,
                shape.fill.as_deref(),
                shape.stroke.as_deref(),
                shape.stroke_width,
                stroke_scale,
            );
        }
        SvgGeometry::Ellipse { cx, cy, rx, ry } => {
            let path = svg_ellipse_path(*cx, *cy, *rx, *ry)?;
            svg_fill_stroke(
                ctx,
                &path,
                shape.fill.as_deref(),
                shape.stroke.as_deref(),
                shape.stroke_width,
                stroke_scale,
            );
        }
        SvgGeometry::Line { x1, y1, x2, y2 } => {
            let d = format!("M{x1},{y1}L{x2},{y2}");
            let path = Path2d::new_with_path_string(&d)?;
            // Lines have no fill by default.
            let stroke = shape.stroke.as_deref().unwrap_or("#000000");
            if stroke != "none" {
                ctx.set_stroke_style_str(stroke);
                let width = shape.stroke_width.unwrap_or(1.0) / stroke_scale;
                ctx.set_line_width(width.max(0.1));
                ctx.stroke_with_path(&path);
            }
        }
        SvgGeometry::Polygon(pts) | SvgGeometry::Polyline(pts) => {
            if pts.len() >= 2 {
                let mut d = format!("M{},{}", pts[0].0, pts[0].1);
                for &(px, py) in &pts[1..] {
                    use std::fmt::Write;
                    let _ = write!(d, "L{px},{py}");
                }
                if matches!(shape.geometry, SvgGeometry::Polygon(_)) {
                    d.push('Z');
                }
                let path = Path2d::new_with_path_string(&d)?;
                svg_fill_stroke(
                    ctx,
                    &path,
                    shape.fill.as_deref(),
                    shape.stroke.as_deref(),
                    shape.stroke_width,
                    stroke_scale,
                );
            }
        }
        SvgGeometry::Text { x, y, content, font_size } => {
            let fill = shape.fill.as_deref().unwrap_or("#000000");
            if fill != "none" {
                ctx.set_fill_style_str(fill);
                let size = font_size.unwrap_or(16.0);
                ctx.set_font(&format!("{size:.0}px sans-serif"));
                ctx.set_text_baseline("auto");
                let _ = ctx.fill_text(content, *x, *y);
            }
        }
        SvgGeometry::Group(children) => {
            ctx.save();
            if let Some(ref tf) = shape.transform {
                apply_svg_transform(ctx, tf)?;
            }
            for child in children {
                render_svg_shape(ctx, child, stroke_scale)?;
            }
            ctx.restore();
            return Ok(());
        }
    }

    Ok(())
}

/// Apply fill and stroke to a `Path2d`.
fn svg_fill_stroke(
    ctx: &CanvasRenderingContext2d,
    path: &Path2d,
    fill: Option<&str>,
    stroke: Option<&str>,
    stroke_width: Option<f64>,
    stroke_scale: f64,
) {
    if fill != Some("none") {
        ctx.set_fill_style_str(fill.unwrap_or("#000000"));
        ctx.fill_with_path_2d(path);
    }
    if let Some(s) = stroke {
        if s != "none" {
            ctx.set_stroke_style_str(s);
            let width = stroke_width.unwrap_or(1.0) / stroke_scale;
            ctx.set_line_width(width.max(0.1));
            ctx.stroke_with_path(path);
        }
    }
}

#[allow(clippy::many_single_char_names)]
fn svg_rect_path(x: f64, y: f64, w: f64, h: f64, rx: f64, ry: f64) -> Result<Path2d, JsValue> {
    if rx > 0.0 || ry > 0.0 {
        let rx = rx.max(ry).min(w / 2.0);
        let ry = ry.max(rx).min(h / 2.0);
        let d = format!(
            "M{},{} h{} a{rx},{ry} 0 0 1 {rx},{ry} v{} a{rx},{ry} 0 0 1 -{rx},{ry} h-{} a{rx},{ry} 0 0 1 -{rx},-{ry} v-{} a{rx},{ry} 0 0 1 {rx},-{ry} Z",
            x + rx,
            y,
            w - 2.0 * rx,
            h - 2.0 * ry,
            w - 2.0 * rx,
            h - 2.0 * ry,
        );
        Path2d::new_with_path_string(&d)
    } else {
        let d = format!("M{x},{y}h{w}v{h}h-{w}Z");
        Path2d::new_with_path_string(&d)
    }
}

fn svg_circle_path(cx: f64, cy: f64, r: f64) -> Result<Path2d, JsValue> {
    let d = format!("M{},{} a{r},{r} 0 1 0 {},0 a{r},{r} 0 1 0 {},0", cx - r, cy, 2.0 * r, -2.0 * r,);
    Path2d::new_with_path_string(&d)
}

fn svg_ellipse_path(cx: f64, cy: f64, rx: f64, ry: f64) -> Result<Path2d, JsValue> {
    let d = format!(
        "M{},{} a{rx},{ry} 0 1 0 {},0 a{rx},{ry} 0 1 0 {},0",
        cx - rx,
        cy,
        2.0 * rx,
        -2.0 * rx,
    );
    Path2d::new_with_path_string(&d)
}

/// Apply a simple SVG `transform` string to the canvas context.
/// Supports: translate(x,y), scale(x,y), rotate(deg), matrix(a,b,c,d,e,f).
fn apply_svg_transform(ctx: &CanvasRenderingContext2d, transform: &str) -> Result<(), JsValue> {
    let mut scan = transform;
    while let Some(paren_start) = scan.find('(') {
        let func_name = scan[..paren_start].trim();
        // Extract the last word if there are spaces (e.g. "foo translate" → "translate").
        let func_name = func_name
            .rsplit_once(char::is_whitespace)
            .map_or(func_name, |(_, n)| n);
        let Some(paren_end) = scan[paren_start..].find(')') else {
            break;
        };
        let args_str = &scan[paren_start + 1..paren_start + paren_end];
        let args: Vec<f64> = args_str
            .split(|c: char| c == ',' || c.is_ascii_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        scan = &scan[paren_start + paren_end + 1..];

        match func_name {
            "translate" => {
                let tx = args.first().copied().unwrap_or(0.0);
                let ty = args.get(1).copied().unwrap_or(0.0);
                ctx.translate(tx, ty)?;
            }
            "scale" => {
                let sx = args.first().copied().unwrap_or(1.0);
                let sy = args.get(1).copied().unwrap_or(sx);
                ctx.scale(sx, sy)?;
            }
            "rotate" => {
                let deg = args.first().copied().unwrap_or(0.0);
                ctx.rotate(deg * PI / 180.0)?;
            }
            "matrix" if args.len() >= 6 => {
                ctx.transform(args[0], args[1], args[2], args[3], args[4], args[5])?;
            }
            _ => {}
        }
    }
    Ok(())
}

// =============================================================
// SVG shape types
// =============================================================

#[derive(Clone, Debug, PartialEq)]
struct SvgShape {
    geometry: SvgGeometry,
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<f64>,
    transform: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum SvgGeometry {
    Path(String),
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rx: f64,
        ry: f64,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Polygon(Vec<(f64, f64)>),
    Polyline(Vec<(f64, f64)>),
    Text {
        x: f64,
        y: f64,
        content: String,
        font_size: Option<f64>,
    },
    Group(Vec<SvgShape>),
}

/// Parse all renderable SVG elements from markup into shapes.
fn parse_svg_shapes(svg: &str) -> Vec<SvgShape> {
    parse_svg_fragment(svg)
}

/// Recursively parse SVG elements from a fragment.
fn parse_svg_fragment(fragment: &str) -> Vec<SvgShape> {
    let mut out = Vec::new();
    let mut scan = fragment;

    while let Some(tag_start) = scan.find('<') {
        let rest = &scan[tag_start..];

        // Skip closing tags, comments, XML declarations, CDATA.
        if rest.starts_with("</") || rest.starts_with("<!") || rest.starts_with("<?") {
            scan = &scan[tag_start + 1..];
            continue;
        }

        // Extract tag name.
        let after_lt = &rest[1..];
        let name_end = after_lt
            .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
            .unwrap_or(after_lt.len());
        let tag_name = &after_lt[..name_end];

        // Find end of opening tag.
        let Some(tag_close) = rest.find('>') else {
            break;
        };
        let tag_str = &rest[..=tag_close];
        let self_closing = tag_str.ends_with("/>") || tag_str.ends_with("/ >");

        match tag_name {
            "path" => {
                if let Some(shape) = parse_path_tag(tag_str) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "rect" => {
                if let Some(shape) = parse_rect_tag(tag_str) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "circle" => {
                if let Some(shape) = parse_circle_tag(tag_str) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "ellipse" => {
                if let Some(shape) = parse_ellipse_tag(tag_str) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "line" => {
                if let Some(shape) = parse_line_tag(tag_str) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "polygon" => {
                if let Some(shape) = parse_poly_tag(tag_str, true) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "polyline" => {
                if let Some(shape) = parse_poly_tag(tag_str, false) {
                    out.push(shape);
                }
                scan = &scan[tag_start + tag_close + 1..];
            }
            "text" => {
                if let Some(shape) = parse_text_tag(tag_str, &scan[tag_start..]) {
                    out.push(shape);
                }
                // Skip past closing </text>.
                let after_open = &scan[tag_start + tag_close + 1..];
                if let Some(close_idx) = after_open.find("</text>") {
                    scan = &after_open[close_idx + 7..];
                } else {
                    scan = &scan[tag_start + tag_close + 1..];
                }
            }
            "g" => {
                let (group_shape, advance) = parse_group_tag(tag_str, self_closing, &scan[tag_start..]);
                if let Some(shape) = group_shape {
                    out.push(shape);
                }
                scan = &scan[tag_start + advance..];
            }
            _ => {
                scan = &scan[tag_start + tag_close + 1..];
            }
        }
    }

    out
}

fn common_style(tag: &str) -> (Option<String>, Option<String>, Option<f64>, Option<String>) {
    (
        attr_value(tag, "fill"),
        attr_value(tag, "stroke"),
        attr_value(tag, "stroke-width").and_then(|v| parse_svg_number(&v)),
        attr_value(tag, "transform"),
    )
}

fn parse_path_tag(tag: &str) -> Option<SvgShape> {
    let d = attr_value(tag, "d")?;
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    Some(SvgShape { geometry: SvgGeometry::Path(d), fill, stroke, stroke_width, transform })
}

fn parse_rect_tag(tag: &str) -> Option<SvgShape> {
    let x = attr_value(tag, "x")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let y = attr_value(tag, "y")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let width = attr_value(tag, "width")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let height = attr_value(tag, "height")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let rx = attr_value(tag, "rx")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let ry = attr_value(tag, "ry")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    if width <= 0.0 && height <= 0.0 {
        return None;
    }
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    Some(SvgShape {
        geometry: SvgGeometry::Rect { x, y, width, height, rx, ry },
        fill,
        stroke,
        stroke_width,
        transform,
    })
}

fn parse_circle_tag(tag: &str) -> Option<SvgShape> {
    let cx = attr_value(tag, "cx")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let cy = attr_value(tag, "cy")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let r = attr_value(tag, "r")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    if r <= 0.0 {
        return None;
    }
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    Some(SvgShape { geometry: SvgGeometry::Circle { cx, cy, r }, fill, stroke, stroke_width, transform })
}

fn parse_ellipse_tag(tag: &str) -> Option<SvgShape> {
    let cx = attr_value(tag, "cx")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let cy = attr_value(tag, "cy")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let rx = attr_value(tag, "rx")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let ry = attr_value(tag, "ry")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    if rx <= 0.0 && ry <= 0.0 {
        return None;
    }
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    Some(SvgShape { geometry: SvgGeometry::Ellipse { cx, cy, rx, ry }, fill, stroke, stroke_width, transform })
}

#[allow(clippy::unnecessary_wraps)]
fn parse_line_tag(tag: &str) -> Option<SvgShape> {
    let x1 = attr_value(tag, "x1")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let y1 = attr_value(tag, "y1")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let x2 = attr_value(tag, "x2")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let y2 = attr_value(tag, "y2")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    Some(SvgShape {
        geometry: SvgGeometry::Line { x1, y1, x2, y2 },
        fill,
        stroke,
        stroke_width: stroke_width.or(Some(1.0)),
        transform,
    })
}

fn parse_points(raw: &str) -> Vec<(f64, f64)> {
    let nums: Vec<f64> = raw
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    nums.chunks(2)
        .filter_map(|chunk| Some((chunk[0], *chunk.get(1)?)))
        .collect()
}

fn parse_poly_tag(tag: &str, is_polygon: bool) -> Option<SvgShape> {
    let pts_str = attr_value(tag, "points")?;
    let pts = parse_points(&pts_str);
    if pts.len() < 2 {
        return None;
    }
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    let geometry = if is_polygon {
        SvgGeometry::Polygon(pts)
    } else {
        SvgGeometry::Polyline(pts)
    };
    Some(SvgShape { geometry, fill, stroke, stroke_width, transform })
}

fn parse_text_tag(tag: &str, full_fragment: &str) -> Option<SvgShape> {
    let x = attr_value(tag, "x")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let y = attr_value(tag, "y")
        .and_then(|v| parse_svg_number(&v))
        .unwrap_or(0.0);
    let font_size = attr_value(tag, "font-size").and_then(|v| parse_svg_number(&v));
    // Extract text content between <text ...> and </text>.
    let content = if let Some(close) = full_fragment.find("</text>") {
        let after_tag = &full_fragment[full_fragment.find('>').map_or(0, |i| i + 1)..close];
        // Strip any nested <tspan> tags but keep text.
        strip_tags(after_tag)
    } else {
        String::new()
    };
    if content.trim().is_empty() {
        return None;
    }
    let (fill, stroke, stroke_width, transform) = common_style(tag);
    Some(SvgShape { geometry: SvgGeometry::Text { x, y, content, font_size }, fill, stroke, stroke_width, transform })
}

/// Strip HTML/SVG tags and return just text content.
fn strip_tags(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

fn parse_group_tag(tag: &str, self_closing: bool, full_fragment: &str) -> (Option<SvgShape>, usize) {
    let (_, _, _, transform) = common_style(tag);
    let (fill, stroke, stroke_width, _) = common_style(tag);

    if self_closing {
        let advance = full_fragment.find('>').map_or(1, |i| i + 1);
        return (None, advance);
    }

    // Find matching </g> — handle nesting.
    let tag_close = full_fragment.find('>').unwrap_or(0);
    let inner_start = tag_close + 1;
    let inner = &full_fragment[inner_start..];
    let close_pos = find_matching_close(inner, "g");
    let inner_content = &inner[..close_pos];
    let advance = inner_start + close_pos + 4; // skip past "</g>"

    let children = parse_svg_fragment(inner_content);
    if children.is_empty() {
        return (None, advance);
    }

    (
        Some(SvgShape { geometry: SvgGeometry::Group(children), fill, stroke, stroke_width, transform }),
        advance,
    )
}

/// Find the position of the matching closing tag, handling nesting.
fn find_matching_close(content: &str, tag_name: &str) -> usize {
    let open_tag = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");
    let mut depth = 1_usize;
    let mut pos = 0;

    while pos < content.len() {
        let next_open = content[pos..].find(&open_tag);
        let next_close = content[pos..].find(&close_tag);

        match (next_open, next_close) {
            (Some(oi), Some(ci)) if oi < ci => {
                // Open tag comes first — check it's not self-closing.
                let check = &content[pos + oi..];
                if let Some(end) = check.find('>') {
                    if !check[..=end].ends_with("/>") {
                        depth += 1;
                    }
                }
                pos += oi + open_tag.len();
            }
            (_, Some(ci)) => {
                // Close tag comes first (or no open tag).
                depth -= 1;
                if depth == 0 {
                    return pos + ci;
                }
                pos += ci + close_tag.len();
            }
            (Some(oi), None) => {
                pos += oi + open_tag.len();
            }
            (None, None) => break,
        }
    }

    content.len()
}

fn svg_view_box(svg: &str) -> Option<(f64, f64, f64, f64)> {
    if let Some(vb) = attr_value(svg, "viewBox") {
        let nums: Vec<f64> = vb
            .split(|c: char| c.is_ascii_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(parse_svg_number)
            .collect();
        if nums.len() == 4 {
            return Some((nums[0], nums[1], nums[2], nums[3]));
        }
    }

    let w = attr_value(svg, "width").and_then(|v| parse_svg_number(&v))?;
    let h = attr_value(svg, "height").and_then(|v| parse_svg_number(&v))?;
    Some((0.0, 0.0, w, h))
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=");
    let start = tag.find(&needle)?;
    let after = &tag[start + needle.len()..];
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_owned())
}

fn parse_svg_number(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    let value = trimmed
        .strip_suffix("px")
        .or_else(|| trimmed.strip_suffix('%'))
        .unwrap_or(trimmed);
    value.parse::<f64>().ok()
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

#[allow(
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
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
                    word.clone_into(&mut current);
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

#[cfg(test)]
mod render_test {
    use super::{SvgGeometry, attr_value, parse_svg_number, parse_svg_shapes, svg_view_box};

    #[test]
    fn parse_svg_shapes_extracts_path_attributes() {
        let svg = r##"<svg viewBox="0 0 100 100"><path d="M0 0 L10 10 Z" fill="#f00" stroke="#000" stroke-width="2"/></svg>"##;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        if let SvgGeometry::Path(ref d) = shapes[0].geometry {
            assert_eq!(d, "M0 0 L10 10 Z");
        } else {
            panic!("expected Path geometry");
        }
        assert_eq!(shapes[0].fill.as_deref(), Some("#f00"));
        assert_eq!(shapes[0].stroke.as_deref(), Some("#000"));
        assert_eq!(shapes[0].stroke_width, Some(2.0));
    }

    #[test]
    fn parse_svg_shapes_extracts_rect() {
        let svg = r##"<svg viewBox="0 0 200 200"><rect x="10" y="20" width="100" height="50" fill="#0f0"/></svg>"##;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        assert!(
            matches!(shapes[0].geometry, SvgGeometry::Rect { x, y, width, height, .. } if x == 10.0 && y == 20.0 && width == 100.0 && height == 50.0)
        );
    }

    #[test]
    fn parse_svg_shapes_extracts_circle() {
        let svg = r#"<svg><circle cx="50" cy="50" r="25" fill="blue"/></svg>"#;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        assert!(
            matches!(shapes[0].geometry, SvgGeometry::Circle { cx, cy, r } if cx == 50.0 && cy == 50.0 && r == 25.0)
        );
    }

    #[test]
    fn parse_svg_shapes_extracts_ellipse() {
        let svg = r#"<svg><ellipse cx="50" cy="50" rx="30" ry="20"/></svg>"#;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        assert!(matches!(shapes[0].geometry, SvgGeometry::Ellipse { .. }));
    }

    #[test]
    fn parse_svg_shapes_extracts_line() {
        let svg = r#"<svg><line x1="0" y1="0" x2="100" y2="100" stroke="black"/></svg>"#;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        assert!(
            matches!(shapes[0].geometry, SvgGeometry::Line { x1, y1, x2, y2 } if x1 == 0.0 && y1 == 0.0 && x2 == 100.0 && y2 == 100.0)
        );
    }

    #[test]
    fn parse_svg_shapes_extracts_polygon() {
        let svg = r#"<svg><polygon points="50,0 100,100 0,100" fill="red"/></svg>"#;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        if let SvgGeometry::Polygon(ref pts) = shapes[0].geometry {
            assert_eq!(pts.len(), 3);
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn parse_svg_shapes_extracts_text() {
        let svg = r#"<svg><text x="10" y="30" font-size="16">Hello</text></svg>"#;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        if let SvgGeometry::Text { x, y, ref content, font_size } = shapes[0].geometry {
            assert_eq!(x, 10.0);
            assert_eq!(y, 30.0);
            assert_eq!(content, "Hello");
            assert_eq!(font_size, Some(16.0));
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn parse_svg_shapes_handles_group() {
        let svg = r#"<svg><g transform="translate(10,20)"><rect x="0" y="0" width="50" height="50"/><circle cx="25" cy="25" r="10"/></g></svg>"#;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 1);
        if let SvgGeometry::Group(ref children) = shapes[0].geometry {
            assert_eq!(children.len(), 2);
        } else {
            panic!("expected Group");
        }
        assert_eq!(shapes[0].transform.as_deref(), Some("translate(10,20)"));
    }

    #[test]
    fn parse_svg_shapes_landscape_like() {
        let svg = r##"<svg viewBox="0 0 400 200">
            <rect x="0" y="100" width="400" height="100" fill="#4a7c5c"/>
            <rect x="0" y="0" width="400" height="100" fill="#87CEEB"/>
            <circle cx="320" cy="40" r="30" fill="#FFD700"/>
            <polygon points="150,100 175,40 200,100" fill="#2d5a2d"/>
            <polygon points="220,100 250,30 280,100" fill="#1e4a1e"/>
        </svg>"##;
        let shapes = parse_svg_shapes(svg);
        assert_eq!(shapes.len(), 5);
        assert!(matches!(shapes[0].geometry, SvgGeometry::Rect { .. }));
        assert!(matches!(shapes[1].geometry, SvgGeometry::Rect { .. }));
        assert!(matches!(shapes[2].geometry, SvgGeometry::Circle { .. }));
        assert!(matches!(shapes[3].geometry, SvgGeometry::Polygon(_)));
        assert!(matches!(shapes[4].geometry, SvgGeometry::Polygon(_)));
    }

    #[test]
    fn svg_view_box_prefers_viewbox() {
        let svg = r#"<svg width="40" height="20" viewBox="2 3 100 50"></svg>"#;
        assert_eq!(svg_view_box(svg), Some((2.0, 3.0, 100.0, 50.0)));
    }

    #[test]
    fn svg_view_box_falls_back_to_dimensions() {
        let svg = r#"<svg width="40px" height="20"></svg>"#;
        assert_eq!(svg_view_box(svg), Some((0.0, 0.0, 40.0, 20.0)));
    }

    #[test]
    fn attr_value_supports_single_or_double_quotes() {
        let tag = "<path d='M0 0' fill=\"#fff\"/>";
        assert_eq!(attr_value(tag, "d").as_deref(), Some("M0 0"));
        assert_eq!(attr_value(tag, "fill").as_deref(), Some("#fff"));
    }

    #[test]
    fn parse_svg_number_handles_px_and_percent() {
        assert_eq!(parse_svg_number("10"), Some(10.0));
        assert_eq!(parse_svg_number("10px"), Some(10.0));
        assert_eq!(parse_svg_number("75%"), Some(75.0));
        assert_eq!(parse_svg_number("x"), None);
    }
}
