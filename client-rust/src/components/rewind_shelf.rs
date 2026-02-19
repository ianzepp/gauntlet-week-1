//! Record-shelf rewind UI at the bottom of the board.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus, Savepoint};
use crate::state::board::BoardState;

#[component]
pub fn RewindShelf() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let hovered_id = RwSignal::new(None::<String>);
    let locked_id = RwSignal::new(None::<String>);

    let ordered = move || {
        let mut rows = board.get().savepoints.clone();
        rows.sort_by(|a, b| a.seq.cmp(&b.seq));
        rows
    };

    let preview = move || {
        let rows = board.get().savepoints.clone();
        let selected = locked_id.get().or_else(|| hovered_id.get());
        selected.and_then(|id| rows.into_iter().find(|s| s.id == id))
    };

    let on_create = move |_| {
        let Some(board_id) = board.get_untracked().board_id else {
            return;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id),
            from: None,
            syscall: "board:savepoint:create".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "label": "Manual savepoint"
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    };

    view! {
        <div class="rewind-shelf">
            <div class="rewind-shelf__header">
                <div class="rewind-shelf__title">"Field Records"</div>
                <button class="rewind-shelf__create" on:click=on_create>
                    "Drop Savepoint"
                </button>
            </div>
            <div class="rewind-shelf__preview">
                {move || {
                    preview().map(|sp| {
                        let dots = snapshot_dots(&sp);
                        view! {
                            <div class="rewind-shelf__preview-card">
                                <div class="rewind-shelf__preview-meta">
                                    <strong>{sp.label.unwrap_or_else(|| "Savepoint".to_owned())}</strong>
                                    <span>{format!("seq {} · {}", sp.seq, sp.reason)}</span>
                                </div>
                                <div class="rewind-shelf__preview-map">
                                    {dots.into_iter().map(|dot| {
                                        view! { <span class=dot.class style=dot.style></span> }
                                    }).collect_view()}
                                </div>
                            </div>
                        }
                            .into_any()
                    }).unwrap_or_else(|| {
                        view! {
                            <div class="rewind-shelf__preview-empty">
                                "Hover a record to preview that checkpoint layout."
                            </div>
                        }
                            .into_any()
                    })
                }}
            </div>
            <div class="rewind-shelf__records">
                {move || {
                    let rows = ordered();
                    let count = rows.len();
                    rows.into_iter().enumerate().map(|(idx, sp)| {
                        let id = sp.id.clone();
                        let active_id = id.clone();
                        let title = sp.label.clone().unwrap_or_else(|| if sp.is_auto { "Auto savepoint".to_owned() } else { "Savepoint".to_owned() });
                        let meta = format!("{} · seq {}", sp.reason, sp.seq);
                        let tilt = record_tilt(idx, count);
                        let is_active = move || {
                            hovered_id.get().as_deref() == Some(active_id.as_str())
                                || locked_id.get().as_deref() == Some(active_id.as_str())
                        };
                        let class_name = move || if is_active() { "rewind-record rewind-record--active" } else { "rewind-record" };

                        view! {
                            <button
                                class=class_name
                                style=format!("--record-tilt: {tilt:.2}deg; --record-z: {};", idx + 1)
                                on:mouseenter={
                                    let id = id.clone();
                                    move |_| hovered_id.set(Some(id.clone()))
                                }
                                on:mouseleave=move |_| hovered_id.set(None)
                                on:click={
                                    let id = id.clone();
                                    move |_| {
                                        if locked_id.get_untracked().as_deref() == Some(id.as_str()) {
                                            locked_id.set(None);
                                        } else {
                                            locked_id.set(Some(id.clone()));
                                        }
                                    }
                                }
                            >
                                <span class="rewind-record__title">{title}</span>
                                <span class="rewind-record__meta">{meta}</span>
                            </button>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}

#[derive(Clone)]
struct PreviewDot {
    class: &'static str,
    style: String,
}

fn snapshot_dots(sp: &Savepoint) -> Vec<PreviewDot> {
    if sp.snapshot.is_empty() {
        return Vec::new();
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for obj in &sp.snapshot {
        let w = obj.width.unwrap_or(80.0).max(24.0);
        let h = obj.height.unwrap_or(80.0).max(24.0);
        let left = obj.x - w * 0.5;
        let top = obj.y - h * 0.5;
        let right = left + w;
        let bottom = top + h;
        min_x = min_x.min(left);
        min_y = min_y.min(top);
        max_x = max_x.max(right);
        max_y = max_y.max(bottom);
    }

    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);

    sp.snapshot
        .iter()
        .take(160)
        .map(|obj| {
            let w = obj.width.unwrap_or(80.0).max(24.0);
            let h = obj.height.unwrap_or(80.0).max(24.0);
            let x = ((obj.x - min_x) / span_x).clamp(0.0, 1.0);
            let y = ((obj.y - min_y) / span_y).clamp(0.0, 1.0);
            let ww = (w / span_x).clamp(0.02, 0.35);
            let hh = (h / span_y).clamp(0.02, 0.35);
            PreviewDot {
                class: preview_class(&obj.kind),
                style: format!(
                    "left: {:.2}%; top: {:.2}%; width: {:.2}%; height: {:.2}%;",
                    x * 100.0,
                    y * 100.0,
                    ww * 100.0,
                    hh * 100.0
                ),
            }
        })
        .collect()
}

fn preview_class(kind: &str) -> &'static str {
    match kind {
        "frame" => "rewind-dot rewind-dot--frame",
        "line" | "arrow" => "rewind-dot rewind-dot--edge",
        _ => "rewind-dot",
    }
}

fn record_tilt(idx: usize, count: usize) -> f64 {
    if count <= 1 {
        return 0.0;
    }
    let norm = idx as f64 / (count - 1) as f64;
    (0.5 - norm) * 18.0
}
