//! Column 3 — DETAIL INSPECTOR for the selected frame.
//!
//! Shows the OVERVIEW tab by default: frame metadata key-value grid plus a
//! JSON data preview.  Mirrors the `CollabBoard` inspector panel's visual
//! vocabulary (monospace, uppercase section labels, dark inset backgrounds).

use leptos::prelude::*;

use crate::state::trace::TraceState;

/// Formats a millisecond epoch timestamp for display.
fn format_ts_full(ms: i64) -> String {
    if ms <= 0 {
        return "—".to_owned();
    }
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;
    format!("{hours:02}:{mins:02}:{secs:02}.{millis:03}")
}

fn status_label(status: frames::Status) -> &'static str {
    match status {
        frames::Status::Request => "request",
        frames::Status::Item => "item",
        frames::Status::Done => "done",
        frames::Status::Error => "error",
        frames::Status::Cancel => "cancel",
    }
}

fn status_class(status: frames::Status) -> &'static str {
    match status {
        frames::Status::Request => "trace-inspector__status trace-inspector__status--request",
        frames::Status::Item => "trace-inspector__status trace-inspector__status--item",
        frames::Status::Done => "trace-inspector__status trace-inspector__status--done",
        frames::Status::Error => "trace-inspector__status trace-inspector__status--error",
        frames::Status::Cancel => "trace-inspector__status trace-inspector__status--cancel",
    }
}

/// Right-side detail inspector for the currently selected frame.
#[component]
pub fn TraceInspector() -> impl IntoView {
    let trace = expect_context::<RwSignal<TraceState>>();

    // Look up the selected frame from the buffer.
    let selected_frame = move || {
        let state = trace.get();
        let frame_id = state.selected_frame_id.as_ref()?;
        state.frames.iter().find(|f| &f.id == frame_id).cloned()
    };

    let on_close = move |_| {
        trace.update(|t| t.selected_frame_id = None);
    };

    view! {
        <div class="trace-inspector">
            <Show
                when=move || selected_frame().is_some()
                fallback=|| view! { <div class="trace-inspector__empty">"Select a frame."</div> }
            >
                {move || {
                    let Some(frame) = selected_frame() else {
                        let _: () = view! { <></> };
                        return ().into_any();
                    };
                    let display = traces::prefix_display(&frame.syscall);
                    let sub = traces::sub_label(&frame);
                    let status_lbl = status_label(frame.status);
                    let status_cls = status_class(frame.status);
                    let data_pretty = serde_json::to_string_pretty(&frame.data)
                        .unwrap_or_else(|_| frame.data.to_string());
                    let id_short = frame.id.chars().take(8).collect::<String>();

                    view! {
                        <div class="trace-inspector__content">

                            // ── Header ───────────────────────────────────────
                            <div class="trace-inspector__header">
                                <div class="trace-inspector__header-row">
                                    <span
                                        class="trace-inspector__prefix-badge"
                                        style=format!("color:{}", display.color)
                                    >
                                        {display.letter}
                                    </span>
                                    <span class="trace-inspector__syscall">{frame.syscall.clone()}</span>
                                    <button
                                        class="trace-inspector__close"
                                        on:click=on_close
                                        title="Close inspector"
                                    >
                                        "✕"
                                    </button>
                                </div>
                                {sub.map(|s| view! {
                                    <div class="trace-inspector__sub-label">{s}</div>
                                })}
                                {frame.from.as_ref().map(|from| view! {
                                    <div class="trace-inspector__from">
                                        <span class="trace-inspector__field-label">"FROM: "</span>
                                        {from.clone()}
                                    </div>
                                })}
                            </div>

                            // ── FRAME_IDENTIFIER ─────────────────────────────
                            <section class="trace-inspector__section">
                                <div class="trace-inspector__section-label">"FRAME_IDENTIFIER / SYSCALL"</div>
                                <div class="trace-inspector__id-card">
                                    <div class="trace-inspector__id-syscall">{frame.syscall.clone()}</div>
                                    {frame.from.as_ref().map(|from| view! {
                                        <div class="trace-inspector__id-from">
                                            <span class="trace-inspector__field-label">"FROM: "</span>
                                            {from.clone()}
                                        </div>
                                    })}
                                </div>
                            </section>

                            // ── FRAME_METRICS ─────────────────────────────────
                            <section class="trace-inspector__section">
                                <div class="trace-inspector__section-label">"FRAME_METRICS"</div>
                                <div class="trace-inspector__kv">
                                    <span class="trace-inspector__field-label">"ID"</span>
                                    <span class="trace-inspector__field-value">{id_short}</span>
                                </div>
                                {frame.parent_id.as_ref().map(|pid| {
                                    let pid_short = pid.chars().take(8).collect::<String>();
                                    view! {
                                        <div class="trace-inspector__kv">
                                            <span class="trace-inspector__field-label">"PARENT_ID"</span>
                                            <span class="trace-inspector__field-value">{pid_short}</span>
                                        </div>
                                    }
                                })}
                                <div class="trace-inspector__kv">
                                    <span class="trace-inspector__field-label">"TS"</span>
                                    <span class="trace-inspector__field-value">
                                        {format_ts_full(frame.ts)}
                                    </span>
                                </div>
                                <div class="trace-inspector__kv">
                                    <span class="trace-inspector__field-label">"STATUS"</span>
                                    <span class=status_cls>{status_lbl}</span>
                                </div>
                                {frame.board_id.as_ref().map(|bid| view! {
                                    <div class="trace-inspector__kv">
                                        <span class="trace-inspector__field-label">"BOARD_ID"</span>
                                        <span class="trace-inspector__field-value">{bid.clone()}</span>
                                    </div>
                                })}
                            </section>

                            // ── DATA_PREVIEW ──────────────────────────────────
                            <section class="trace-inspector__section">
                                <div class="trace-inspector__section-label">"DATA_PREVIEW"</div>
                                <pre class="trace-inspector__json">{data_pretty}</pre>
                            </section>
                        </div>
                    }.into_any()
                }}
            </Show>
        </div>
    }
}
