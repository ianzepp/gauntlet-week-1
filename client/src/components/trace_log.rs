//! Column 2 — EVENT LOG (flat chronological frame list).
//!
//! Renders the frames for the currently selected trace session (or all
//! buffered frames in live mode) as a scrollable flat log.  Clicking a row
//! selects that frame and opens the detail inspector in Column 3.

use leptos::prelude::*;
use std::collections::HashMap;

use crate::state::trace::TraceState;
use crate::state::ui::{RightTab, UiState, ViewMode};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortColumn {
    Time,
    Type,
    Syscall,
    Status,
    From,
}

#[derive(Clone, Debug)]
struct TraceRow {
    id: String,
    ts_ms: i64,
    ts: String,
    letter: &'static str,
    color: &'static str,
    syscall: String,
    sub: Option<String>,
    status_text: &'static str,
    status_mod: &'static str,
    from: String,
}

fn status_rank(status_mod: &str) -> u8 {
    match status_mod {
        "request" => 0,
        "item" => 1,
        "done" => 2,
        "error" => 3,
        "cancel" => 4,
        _ => 255,
    }
}

/// Computes display depth for the trace log tree.
///
/// When a session root is selected, children directly under that root are
/// rendered as top-level rows (depth 0) so the log does not visually indent
/// every row under the hidden/root frame.
fn display_depth(frame_id: &str, by_id: &HashMap<String, frames::Frame>, selected_session_id: Option<&str>) -> usize {
    let Some(session_root_id) = selected_session_id else {
        return traces::tree_depth(frame_id, by_id);
    };
    if frame_id == session_root_id {
        return 0;
    }

    let mut depth = 0usize;
    let mut current = frame_id;
    let mut seen = std::collections::HashSet::<String>::new();

    while let Some(frame) = by_id.get(current) {
        let Some(parent) = frame.parent_id.as_deref() else {
            break;
        };
        if parent == session_root_id {
            break;
        }
        if !seen.insert(parent.to_owned()) {
            break;
        }
        if !by_id.contains_key(parent) {
            break;
        }
        depth += 1;
        current = parent;
    }

    depth
}

/// Center column flat event log.
#[component]
pub fn TraceLog() -> impl IntoView {
    let trace = expect_context::<RwSignal<TraceState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let sort_column = RwSignal::new(SortColumn::Time);
    let sort_desc = RwSignal::new(false);

    let set_sort = move |column: SortColumn| {
        if sort_column.get_untracked() == column {
            sort_desc.update(|v| *v = !*v);
        } else {
            sort_column.set(column);
            sort_desc.set(false);
        }
    };

    let selected_frame_id = move || trace.get().selected_frame_id.clone();

    // Frames to display — from the selected session, filtered by current filter.
    let rows = move || {
        let state = trace.get();
        let session_frames = state.session_frames();
        let visible_frames = session_frames
            .into_iter()
            .filter(|f| state.filter.allows(f))
            .collect::<Vec<_>>();
        let by_id: HashMap<String, frames::Frame> = visible_frames
            .iter()
            .map(|f| (f.id.clone(), (*f).clone()))
            .collect();
        visible_frames
            .into_iter()
            .map(|f| {
                let display = traces::prefix_display(&f.syscall);
                let sub = traces::sub_label(f);
                let (status_text, status_mod) = status_label(f.status);
                let depth = display_depth(&f.id, &by_id, state.selected_session_id.as_deref());
                let visual_depth = depth.saturating_sub(1);
                let tree_indent = if visual_depth == 0 {
                    String::new()
                } else {
                    format!("{}└─ ", "  ".repeat(visual_depth))
                };
                TraceRow {
                    id: f.id.clone(),
                    ts_ms: f.ts,
                    ts: format_ts(f.ts),
                    letter: display.letter,
                    color: display.color,
                    syscall: format!("{tree_indent}{}", f.syscall),
                    sub,
                    status_text,
                    status_mod,
                    from: f.from.clone().unwrap_or_default(),
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="trace-log">
            <div class="trace-log__header">
                <button class="trace-log__head-btn trace-log__col trace-log__col--ts" on:click=move |_| set_sort(SortColumn::Time)>
                    "TIME"
                    {move || if sort_column.get() == SortColumn::Time { if sort_desc.get() { " ↓" } else { " ↑" } } else { "" }}
                </button>
                <button class="trace-log__head-btn trace-log__col trace-log__col--badge" on:click=move |_| set_sort(SortColumn::Type)>
                    "T"
                    {move || if sort_column.get() == SortColumn::Type { if sort_desc.get() { " ↓" } else { " ↑" } } else { "" }}
                </button>
                <button class="trace-log__head-btn trace-log__col trace-log__col--syscall" on:click=move |_| set_sort(SortColumn::Syscall)>
                    "SYSCALL"
                    {move || if sort_column.get() == SortColumn::Syscall { if sort_desc.get() { " ↓" } else { " ↑" } } else { "" }}
                </button>
                <button class="trace-log__head-btn trace-log__col trace-log__col--status" on:click=move |_| set_sort(SortColumn::Status)>
                    "STATUS"
                    {move || if sort_column.get() == SortColumn::Status { if sort_desc.get() { " ↓" } else { " ↑" } } else { "" }}
                </button>
                <button class="trace-log__head-btn trace-log__col trace-log__col--from" on:click=move |_| set_sort(SortColumn::From)>
                    "FROM"
                    {move || if sort_column.get() == SortColumn::From { if sort_desc.get() { " ↓" } else { " ↑" } } else { "" }}
                </button>
            </div>
            <div class="trace-log__rows">
                {move || {
                    let mut all_rows = rows();
                    if all_rows.is_empty() {
                        return view! {
                            <div class="trace-log__empty">
                                "No frames yet. Frames appear as the WebSocket receives them."
                            </div>
                        }.into_any();
                    }
                    let active_sort = sort_column.get();
                    let is_desc = sort_desc.get();
                    all_rows.sort_by(|a, b| {
                        let cmp = match active_sort {
                            SortColumn::Time => a.ts_ms.cmp(&b.ts_ms).then_with(|| a.id.cmp(&b.id)),
                            SortColumn::Type => a.letter.cmp(b.letter).then_with(|| a.ts_ms.cmp(&b.ts_ms)),
                            SortColumn::Syscall => a.syscall.cmp(&b.syscall).then_with(|| a.ts_ms.cmp(&b.ts_ms)),
                            SortColumn::Status => status_rank(a.status_mod)
                                .cmp(&status_rank(b.status_mod))
                                .then_with(|| a.ts_ms.cmp(&b.ts_ms)),
                            SortColumn::From => a.from.cmp(&b.from).then_with(|| a.ts_ms.cmp(&b.ts_ms)),
                        };
                        if is_desc { cmp.reverse() } else { cmp }
                    });
                    let sel = selected_frame_id();
                    all_rows.into_iter().map(|row| {
                        let is_active = sel.as_deref() == Some(row.id.as_str());
                        let id_clone = row.id.clone();
                        view! {
                            <button
                                class="trace-log__row"
                                class:trace-log__row--active=is_active
                                on:click=move |_| {
                                    let frame_id = id_clone.clone();
                                    trace.update(|t| t.selected_frame_id = Some(frame_id));
                                    ui.update(|u| {
                                        if u.view_mode == ViewMode::Trace {
                                            u.right_panel_expanded = true;
                                            u.right_tab = RightTab::Trace;
                                        }
                                    });
                                }
                            >
                                <span class="trace-log__col trace-log__col--ts trace-log__ts">{row.ts}</span>
                                <span
                                    class="trace-log__col trace-log__col--badge trace-log__badge"
                                    style=format!("color:{}", row.color)
                                >
                                    {row.letter}
                                </span>
                                <span class="trace-log__col trace-log__col--syscall">
                                    <span class="trace-log__syscall">{row.syscall}</span>
                                    {row.sub.map(|s| view! {
                                        <span class="trace-log__sub-label">{s}</span>
                                    })}
                                </span>
                                <span class=format!(
                                    "trace-log__col trace-log__col--status trace-log__status trace-log__status--{}",
                                    row.status_mod
                                )>
                                    {row.status_text}
                                </span>
                                <span class="trace-log__col trace-log__col--from trace-log__from">
                                    {row.from}
                                </span>
                            </button>
                        }
                    }).collect_view().into_any()
                }}
            </div>
        </div>
    }
}
