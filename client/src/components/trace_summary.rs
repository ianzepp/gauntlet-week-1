//! Column 1 — TRACE SUMMARY panel.
//!
//! Renders live aggregate metrics, per-prefix frame counts, connection
//! status, and a scrollable TRACE_INDEX list of available trace sessions.

use leptos::prelude::*;
use std::collections::BTreeMap;

use crate::state::board::{BoardState, ConnectionStatus};
use crate::state::trace::TraceState;

fn session_activity_summary(session: &traces::TraceSession) -> String {
    let mut by_prefix = BTreeMap::<String, usize>::new();
    for frame in &session.frames {
        let prefix = traces::syscall_prefix(&frame.syscall);
        let key = if prefix.is_empty() {
            "other".to_owned()
        } else {
            prefix.to_owned()
        };
        *by_prefix.entry(key).or_insert(0) += 1;
    }

    let mut ops = by_prefix.into_iter().collect::<Vec<_>>();
    ops.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let top_ops = ops
        .into_iter()
        .take(3)
        .map(|(prefix, count)| {
            let short = match prefix.as_str() {
                "object" => "obj",
                "tool" => "tool",
                "ai" => "ai",
                "board" => "board",
                "chat" => "chat",
                "save" => "save",
                "cursor" => "cursor",
                _ => "other",
            };
            format!("{short}:{count}")
        })
        .collect::<Vec<_>>()
        .join(" · ");

    let mut summary = if top_ops.is_empty() {
        "ops:0".to_owned()
    } else {
        top_ops
    };

    let errors = session.error_count();
    if errors > 0 {
        summary.push_str(&format!(" · err:{errors}"));
    }

    let tokens = session.total_tokens();
    if tokens > 0 {
        summary.push_str(&format!(" · tok:{tokens}"));
    }

    summary
}

/// Left summary panel showing aggregate metrics, prefix breakdown, and
/// the selectable trace session index.
#[component]
pub fn TraceSummary() -> impl IntoView {
    let trace = expect_context::<RwSignal<TraceState>>();
    let board = expect_context::<RwSignal<BoardState>>();

    // Top-level counts derived from the full (unfiltered) buffer.
    let total_frames = move || trace.get().total_frames();
    let error_count = move || trace.get().error_count();

    // Per-prefix breakdown from compute_metrics.
    let metrics = move || traces::compute_metrics(&trace.get().frames);

    let board_name = move || {
        board
            .get()
            .board_name
            .clone()
            .unwrap_or_else(|| "—".to_owned())
    };
    let board_id = move || {
        board
            .get()
            .board_id
            .clone()
            .unwrap_or_else(|| "—".to_owned())
    };

    let connection_label = move || match board.get().connection_status {
        ConnectionStatus::Connected => "● WEBSOCKET OPEN",
        ConnectionStatus::Connecting => "○ CONNECTING",
        ConnectionStatus::Disconnected => "□ DISCONNECTED",
    };

    let connection_class = move || match board.get().connection_status {
        ConnectionStatus::Connected => "trace-summary__conn-status trace-summary__conn-status--open",
        ConnectionStatus::Connecting => "trace-summary__conn-status trace-summary__conn-status--connecting",
        ConnectionStatus::Disconnected => "trace-summary__conn-status trace-summary__conn-status--closed",
    };

    // Build the trace session index from the buffered frames.
    let sessions = move || {
        let state = trace.get();
        let filter = state.filter.clone();
        traces::build_trace_sessions(&state.frames)
            .into_iter()
            .filter(|session| session.frames.iter().any(|frame| filter.allows(frame)))
            .collect::<Vec<_>>()
    };

    let selected_session_id = move || trace.get().selected_session_id.clone();

    view! {
        <div class="trace-summary">

            // ── TRACE_ACTIVITY ──────────────────────────────────────────────
            <section class="trace-summary__section">
                <div class="trace-summary__section-label">"TRACE_ACTIVITY"</div>
                <div class="trace-summary__metric">
                    <span class="trace-summary__metric-value">{total_frames}</span>
                    <span class="trace-summary__metric-label">"FRAMES_RECEIVED"</span>
                </div>
            </section>

            // ── FRAME_STATE ─────────────────────────────────────────────────
            <section class="trace-summary__section">
                <div class="trace-summary__section-label">"FRAME_STATE"</div>
                {move || {
                    let m = metrics();
                    let mut rows = m.by_prefix.iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect::<Vec<_>>();
                    rows.sort_by(|a, b| b.1.cmp(&a.1));
                    rows.into_iter().map(|(prefix, count)| {
                        view! {
                            <div class="trace-summary__kv">
                                <span class="trace-summary__kv-key">{format!("{prefix}:*")}</span>
                                <span class="trace-summary__kv-value">{count}</span>
                            </div>
                        }
                    }).collect_view()
                }}
                <div class="trace-summary__divider"></div>
                <div class="trace-summary__kv">
                    <span class="trace-summary__kv-key">"ERRORS:"</span>
                    <span class="trace-summary__kv-value">{error_count}</span>
                </div>
            </section>

            // ── CONNECTION_STATUS ────────────────────────────────────────────
            <section class="trace-summary__section">
                <div class="trace-summary__section-label">"CONNECTION_STATUS"</div>
                <div class=connection_class>{connection_label}</div>
                <div class="trace-summary__kv">
                    <span class="trace-summary__kv-key">"FRAMES:"</span>
                    <span class="trace-summary__kv-value">{total_frames}</span>
                </div>
                <div class="trace-summary__kv">
                    <span class="trace-summary__kv-key">"BOARD:"</span>
                    <span class="trace-summary__kv-value">{board_id}</span>
                </div>
            </section>

            // ── ACTIVE_TRACE_CONTEXT ─────────────────────────────────────────
            <section class="trace-summary__section">
                <div class="trace-summary__context-card">
                    <div class="trace-summary__section-label">"ACTIVE_TRACE_CONTEXT"</div>
                    <div class="trace-summary__kv">
                        <span class="trace-summary__kv-key">"BOARD:"</span>
                        <span class="trace-summary__kv-value">{board_name}</span>
                    </div>
                    {move || {
                        let sel = selected_session_id();
                        let root = sel.as_deref()
                            .map(|id| &id[..id.len().min(8)])
                            .unwrap_or("—")
                            .to_owned();
                        view! {
                            <div class="trace-summary__kv">
                                <span class="trace-summary__kv-key">"ROOT:"</span>
                                <span class="trace-summary__kv-value">{root}</span>
                            </div>
                        }
                    }}
                </div>
            </section>

            // ── TRACE_INDEX ──────────────────────────────────────────────────
            <section class="trace-summary__section trace-summary__section--index">
                <div class="trace-summary__section-label">"TRACE_INDEX"</div>
                <div class="trace-summary__index-list">
                    {move || {
                        let all_sessions = sessions();
                        let sel = selected_session_id();
                        if all_sessions.is_empty() {
                            return view! {
                                <div class="trace-summary__index-empty">"No sessions yet."</div>
                            }.into_any();
                        }
                        all_sessions.into_iter().rev().map(|session| {
                            let id = session.root_frame_id.clone();
                            let id_short = id.chars().take(8).collect::<String>();
                            let is_active = sel.as_deref() == Some(id.as_str());
                            let frame_count = session.total_frames();
                            let duration_label = session.ended_at
                                .map(|end| {
                                    let ms = end - session.started_at;
                                    format!("{:.1}s", ms as f64 / 1000.0)
                                })
                                .unwrap_or_else(|| "live".to_owned());
                            let summary = session_activity_summary(&session);
                            let id_clone = id.clone();
                            view! {
                                <button
                                    class="trace-summary__index-row"
                                    class:trace-summary__index-row--active=is_active
                                    on:click=move |_| {
                                        let session_id = id_clone.clone();
                                        trace.update(|t| {
                                            t.selected_session_id = Some(session_id);
                                            t.selected_frame_id = None;
                                        });
                                    }
                                >
                                    <div class="trace-summary__index-main">
                                        <span class="trace-summary__index-dot">
                                            {if is_active { "●" } else { "○" }}
                                        </span>
                                        <span class="trace-summary__index-id">{id_short}</span>
                                        <span class="trace-summary__index-count">
                                            {format!("{frame_count}f")}
                                        </span>
                                        <span class="trace-summary__index-duration">{duration_label}</span>
                                    </div>
                                    <div class="trace-summary__index-meta">{summary}</div>
                                </button>
                            }
                        }).collect_view().into_any()
                    }}
                </div>
            </section>
        </div>
    }
}
