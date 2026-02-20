//! Column 2 — EVENT LOG (flat chronological frame list).
//!
//! Renders the frames for the currently selected trace session (or all
//! buffered frames in live mode) as a scrollable flat log.  Clicking a row
//! selects that frame and opens the detail inspector in Column 3.

use leptos::prelude::*;
use std::collections::HashMap;

use crate::state::trace::TraceState;

/// Formats a millisecond epoch timestamp as `HH:MM:SS.mmm`.
fn format_ts(ms: i64) -> String {
    if ms <= 0 {
        return "--:--:--.---".to_owned();
    }
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;
    format!("{hours:02}:{mins:02}:{secs:02}.{millis:03}")
}

/// Status badge label and CSS modifier.
fn status_label(status: frames::Status) -> (&'static str, &'static str) {
    match status {
        frames::Status::Request => ("req", "request"),
        frames::Status::Item => ("item", "item"),
        frames::Status::Done => ("done", "done"),
        frames::Status::Error => ("err", "error"),
        frames::Status::Cancel => ("cancel", "cancel"),
    }
}

/// Center column flat event log.
#[component]
pub fn TraceLog() -> impl IntoView {
    let trace = expect_context::<RwSignal<TraceState>>();

    let selected_frame_id = move || trace.get().selected_frame_id.clone();

    // Frames to display — from the selected session, filtered by current filter.
    let rows = move || {
        let state = trace.get();
        let session_frames = state.session_frames();
        let by_id: HashMap<String, frames::Frame> = session_frames
            .iter()
            .map(|f| (f.id.clone(), (*f).clone()))
            .collect();
        session_frames
            .into_iter()
            .filter(|f| state.filter.allows(f))
            .map(|f| {
                let display = traces::prefix_display(&f.syscall);
                let sub = traces::sub_label(f);
                let (status_text, status_mod) = status_label(f.status);
                let depth = traces::tree_depth(&f.id, &by_id);
                let tree_indent = if depth == 0 {
                    String::new()
                } else {
                    format!("{}└─ ", "  ".repeat(depth))
                };
                (
                    f.id.clone(),
                    format_ts(f.ts),
                    display.letter,
                    display.color,
                    format!("{tree_indent}{}", f.syscall),
                    sub,
                    status_text,
                    status_mod,
                    f.from.clone().unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="trace-log">
            <div class="trace-log__header">
                <span class="trace-log__col trace-log__col--ts">"TIME"</span>
                <span class="trace-log__col trace-log__col--badge">"T"</span>
                <span class="trace-log__col trace-log__col--syscall">"SYSCALL"</span>
                <span class="trace-log__col trace-log__col--status">"STATUS"</span>
                <span class="trace-log__col trace-log__col--from">"FROM"</span>
            </div>
            <div class="trace-log__rows">
                {move || {
                    let all_rows = rows();
                    if all_rows.is_empty() {
                        return view! {
                            <div class="trace-log__empty">
                                "No frames yet. Frames appear as the WebSocket receives them."
                            </div>
                        }.into_any();
                    }
                    let sel = selected_frame_id();
                    all_rows.into_iter().map(|(id, ts, letter, color, syscall, sub, status_text, status_mod, from)| {
                        let is_active = sel.as_deref() == Some(id.as_str());
                        let id_clone = id.clone();
                        view! {
                            <button
                                class="trace-log__row"
                                class:trace-log__row--active=is_active
                                on:click=move |_| {
                                    let frame_id = id_clone.clone();
                                    trace.update(|t| t.selected_frame_id = Some(frame_id));
                                }
                            >
                                <span class="trace-log__col trace-log__col--ts trace-log__ts">{ts}</span>
                                <span
                                    class="trace-log__col trace-log__col--badge trace-log__badge"
                                    style=format!("color:{color}")
                                >
                                    {letter}
                                </span>
                                <span class="trace-log__col trace-log__col--syscall">
                                    <span class="trace-log__syscall">{syscall}</span>
                                    {sub.map(|s| view! {
                                        <span class="trace-log__sub-label">{s}</span>
                                    })}
                                </span>
                                <span class=format!(
                                    "trace-log__col trace-log__col--status trace-log__status trace-log__status--{status_mod}"
                                )>
                                    {status_text}
                                </span>
                                <span class="trace-log__col trace-log__col--from trace-log__from">
                                    {from}
                                </span>
                            </button>
                        }
                    }).collect_view().into_any()
                }}
            </div>
        </div>
    }
}
