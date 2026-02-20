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
//!
//! Frame detail inspection is rendered in the existing right rail panel.

use leptos::prelude::*;

use crate::components::trace_log::TraceLog;
use crate::components::trace_summary::TraceSummary;

/// Root component for the observability trace view.
#[component]
pub fn TraceView() -> impl IntoView {
    view! {
        <div class="trace-view">
            <div class="trace-view__col trace-view__col--summary">
                <TraceSummary/>
            </div>
            <div class="trace-view__col trace-view__col--log">
                <TraceLog/>
            </div>
        </div>
    }
}
