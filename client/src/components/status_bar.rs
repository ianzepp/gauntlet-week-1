//! Bottom status bar showing connection status and canvas telemetry.
//!
//! SYSTEM CONTEXT
//! ==============
//! Renders low-frequency board telemetry so users can monitor connection and
//! viewport state without opening additional panels.

use leptos::prelude::*;

use crate::net::types::Point;
use crate::state::board::{BoardState, ConnectionStatus};
use crate::state::canvas_view::CanvasViewState;
use crate::state::trace::TraceState;
use crate::state::ui::{UiState, ViewMode};

/// Status bar at the bottom of the board page.
#[component]
pub fn StatusBar(on_help: Callback<()>) -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let canvas_view = expect_context::<RwSignal<CanvasViewState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let trace = expect_context::<RwSignal<TraceState>>();

    let status_class = move || {
        let status = board.get().connection_status;
        match status {
            ConnectionStatus::Connected => "status-bar__dot status-bar__dot--connected",
            ConnectionStatus::Connecting => "status-bar__dot status-bar__dot--connecting",
            ConnectionStatus::Disconnected => "status-bar__dot status-bar__dot--disconnected",
        }
    };

    let object_count = move || board.get().objects.len();
    let camera_locked = move || board.get().follow_client_id.is_some();
    let cursor = move || canvas_view.get().cursor_world;
    let camera_center = move || canvas_view.get().camera_center_world.clone();
    let fps = move || canvas_view.get().fps;

    let is_trace_mode = move || ui.get().view_mode == ViewMode::Trace;

    let trace_frame_count = move || trace.get().total_frames();
    let trace_filter_label = move || {
        let state = trace.get();
        let prefixes = state.filter.active_prefixes();
        if prefixes.len() >= 5 {
            "ALL".to_owned()
        } else {
            prefixes.join("+")
        }
    };
    let trace_mode_label = move || {
        if trace.get().paused { "PAUSED" } else { "LIVE ●" }
    };
    let on_help_click = move |_| on_help.run(());

    view! {
        <div class="status-bar">
            // Left section: connection + mode-specific board telemetry
            <div class="status-bar__section">
                <span class="status-bar__item">
                    <span class=status_class></span>
                </span>
                <span class="status-bar__divider"></span>
                <button class="status-bar__help" on:click=on_help_click title="Open help">"[?] HELP"</button>

                <Show
                    when=is_trace_mode
                    fallback=move || view! {
                        <>
                            <span class="status-bar__divider"></span>
                            <span class="status-bar__item">
                                {move || format!("{} objs", object_count())}
                            </span>
                            <Show when=camera_locked>
                                <span class="status-bar__divider"></span>
                                <span class="status-bar__item status-bar__item--locked">"LOCKED"</span>
                            </Show>
                        </>
                    }
                >
                    <>
                        <span class="status-bar__divider"></span>
                        <span class="status-bar__item">
                            {"TRACE | "}
                            {trace_frame_count}
                            {" FRAMES | FILTER: "}
                            {trace_filter_label}
                            {" | MODE: "}
                            {trace_mode_label}
                        </span>
                    </>
                </Show>
            </div>

            // Right section: canvas telemetry (hidden in trace mode)
            <Show when=move || !is_trace_mode()>
                <div class="status-bar__section">
                    <span class="status-bar__item">{move || format_cursor(cursor())}</span>

                    <span class="status-bar__divider"></span>
                    <span class="status-bar__item">{move || format_point(camera_center())}</span>

                    <span class="status-bar__divider"></span>
                    <span class="status-bar__item">{move || format_fps(fps())}</span>
                </div>
            </Show>
        </div>
    }
}

fn format_cursor(point: Option<Point>) -> String {
    point.map_or_else(|| "(-, -)".to_owned(), format_point)
}

fn format_point(point: Point) -> String {
    format!("({}, {})", round_coord(point.x), round_coord(point.y))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn format_fps(fps: Option<f64>) -> String {
    match fps {
        Some(value) => format!("{} fps", value.round() as i64),
        None => "-- fps".to_owned(),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn round_coord(value: f64) -> i64 {
    value.round() as i64
}
