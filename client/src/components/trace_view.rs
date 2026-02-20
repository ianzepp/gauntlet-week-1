//! Three-column observability trace view.
//!
//! LAYOUT
//! ======
//! Replaces the canvas area when the toolbar TRACE toggle is active.
//! Mirrors the CollabBoard inspector's visual vocabulary (dark theme,
//! monospace labels) applied to the LangSmith-style three-column layout
//! described in `collabboard-observability-design.md`.
//!
//!   Col 1 (~240px) — TRACE SUMMARY  (metrics, session index)
//!   Col 2 (flex-1) — EVENT LOG      (flat chronological frame list)
//!   Col 3 (~400px) — DETAIL         (frame inspector, shown on selection)

use leptos::prelude::*;

use crate::components::trace_inspector::TraceInspector;
use crate::components::trace_log::TraceLog;
use crate::components::trace_summary::TraceSummary;
use crate::state::trace::TraceState;

/// Root component for the observability trace view.
#[component]
pub fn TraceView() -> impl IntoView {
    let trace = expect_context::<RwSignal<TraceState>>();
    let has_selection = move || trace.get().selected_frame_id.is_some();

    view! {
        <div class="trace-view">
            <div class="trace-view__col trace-view__col--summary">
                <TraceSummary/>
            </div>
            <div class="trace-view__col trace-view__col--log">
                <TraceLog/>
            </div>
            <Show when=has_selection>
                <div class="trace-view__col trace-view__col--inspector">
                    <TraceInspector/>
                </div>
            </Show>
        </div>
    }
}
