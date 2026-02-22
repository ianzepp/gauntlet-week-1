//! Reusable card component for board list items on the dashboard.
//!
//! DESIGN
//! ======
//! Keeps board list presentation consistent between dashboard and mission
//! control while centralizing navigation affordances.

use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast as _;

use crate::state::boards::BoardListPreviewObject;

/// A clickable card representing a board.
#[component]
pub fn BoardCard(
    id: String,
    name: String,
    #[prop(default = Vec::new())] snapshot: Vec<BoardListPreviewObject>,
    #[prop(optional)] active: bool,
    #[prop(optional)] mini: bool,
    #[prop(optional)] on_delete: Option<Callback<String>>,
) -> impl IntoView {
    let href = format!("/board/{id}");
    let preview_ref = NodeRef::<leptos::html::Canvas>::new();
    let snapshot_count = snapshot.len();
    let on_delete_click = Callback::new({
        let id = id.clone();
        move |()| {
            if let Some(on_delete) = on_delete.as_ref() {
                on_delete.run(id.clone());
            }
        }
    });
    #[cfg(feature = "hydrate")]
    {
        let preview_ref = preview_ref.clone();
        let snapshot = snapshot.clone();
        Effect::new(move || {
            let Some(canvas) = preview_ref.get() else {
                return;
            };
            draw_preview_canvas(&canvas, &snapshot);
        });
    }

    view! {
        <a
            class="board-card"
            class:board-card--active=active
            class:board-card--mini=mini
            href=href
        >
            <span class="board-card__name">{name}</span>
            <span class="board-card__id">{id}</span>
            <Show when=move || !mini>
                <button
                    class="board-card__delete"
                    on:click=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        on_delete_click.run(());
                    }
                    title="Delete board"
                    aria-label="Delete board"
                >
                    "✕"
                </button>
            </Show>
            <span class="board-card__preview">
                <canvas class="board-card__preview-canvas" node_ref=preview_ref aria-hidden="true"></canvas>
                <span class="board-card__preview-meta">{format!("{snapshot_count} items")}</span>
            </span>
        </a>
    }
}

#[cfg(feature = "hydrate")]
fn draw_preview_canvas(canvas: &web_sys::HtmlCanvasElement, snapshot: &[BoardListPreviewObject]) {
    let width_css = f64::from(canvas.client_width().max(1));
    let height_css = f64::from(canvas.client_height().max(1));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        canvas.set_width(width_css.round() as u32);
        canvas.set_height(height_css.round() as u32);
    }

    let Some(ctx_value) = canvas.get_context("2d").ok().flatten() else {
        return;
    };
    let Ok(ctx) = ctx_value.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
        return;
    };

    ctx.set_fill_style_str("#f6f1e7");
    ctx.fill_rect(0.0, 0.0, width_css, height_css);
    ctx.set_stroke_style_str("#cec3b4");
    ctx.stroke_rect(0.5, 0.5, (width_css - 1.0).max(0.0), (height_css - 1.0).max(0.0));

    if snapshot.is_empty() {
        return;
    }

    let Some((min_x, min_y, max_x, max_y)) = snapshot_bounds(snapshot) else {
        return;
    };
    let bounds_w = (max_x - min_x).max(1.0);
    let bounds_h = (max_y - min_y).max(1.0);
    let padding = 8.0;
    let draw_w = (width_css - (padding * 2.0)).max(1.0);
    let draw_h = (height_css - (padding * 2.0)).max(1.0);
    let scale = (draw_w / bounds_w).min(draw_h / bounds_h);
    let offset_x = ((width_css - (bounds_w * scale)) * 0.5) - (min_x * scale);
    let offset_y = ((height_css - (bounds_h * scale)) * 0.5) - (min_y * scale);

    let mut ordered = snapshot.to_vec();
    ordered.sort_by_key(|obj| obj.z_index);

    for obj in &ordered {
        let (w, h) = preview_size(obj);
        let x = offset_x + (obj.x * scale);
        let y = offset_y + (obj.y * scale);
        let w_px = w * scale;
        let h_px = h * scale;

        let kind = obj.kind.to_ascii_lowercase();
        if matches!(kind.as_str(), "line" | "arrow") {
            ctx.begin_path();
            ctx.set_stroke_style_str("#3d3428");
            ctx.set_line_width(1.0);
            ctx.move_to(x, y);
            ctx.line_to(x + w_px, y + h_px);
            ctx.stroke();
            continue;
        }

        let _ = ctx.save();
        let cx = x + (w_px * 0.5);
        let cy = y + (h_px * 0.5);
        let _ = ctx.translate(cx, cy);
        let _ = ctx.rotate(obj.rotation.to_radians());

        let fill = preview_fill_color(&kind);
        ctx.set_fill_style_str(fill);
        ctx.fill_rect(-w_px * 0.5, -h_px * 0.5, w_px, h_px);
        ctx.set_stroke_style_str("#3d3428");
        ctx.set_line_width(1.0);
        ctx.stroke_rect(-w_px * 0.5, -h_px * 0.5, w_px, h_px);
        let _ = ctx.restore();
    }
}

#[cfg(feature = "hydrate")]
fn snapshot_bounds(snapshot: &[BoardListPreviewObject]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for obj in snapshot {
        let (w, h) = preview_size(obj);
        let x2 = obj.x + w;
        let y2 = obj.y + h;
        min_x = min_x.min(obj.x.min(x2));
        min_y = min_y.min(obj.y.min(y2));
        max_x = max_x.max(obj.x.max(x2));
        max_y = max_y.max(obj.y.max(y2));
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

#[cfg(feature = "hydrate")]
fn preview_size(obj: &BoardListPreviewObject) -> (f64, f64) {
    let kind = obj.kind.to_ascii_lowercase();
    if matches!(kind.as_str(), "line" | "arrow") {
        return (obj.width.unwrap_or(80.0), obj.height.unwrap_or(0.0));
    }
    (obj.width.unwrap_or(160.0).max(8.0), obj.height.unwrap_or(110.0).max(8.0))
}

#[cfg(feature = "hydrate")]
fn preview_fill_color(kind: &str) -> &'static str {
    match kind {
        "sticky_note" => "#f6d87c",
        "frame" => "#d6dbe4",
        "ellipse" => "#bdd7ff",
        _ => "#d5cfbf",
    }
}
