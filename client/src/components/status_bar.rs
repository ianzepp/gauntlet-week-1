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

    let status_class = move || connection_status_class(board.get().connection_status);

    let object_count = move || board_object_count(&board.get());
    let join_round_trip_ms = move || board_join_ms(&board.get());
    let camera_locked = move || board_camera_locked(&board.get());
    let cursor = move || canvas_cursor(&canvas_view.get());
    let camera_center = move || canvas_center(&canvas_view.get());
    let fps = move || canvas_fps(&canvas_view.get());
    let render_ms = move || canvas_render_ms(&canvas_view.get());

    let is_trace_mode = move || ui_is_trace_mode(&ui.get());

    let trace_frame_count = move || trace_frame_total(&trace.get());
    let trace_filter_label = move || trace_filter_display(&trace.get());
    let trace_mode_label = move || trace_mode_display(&trace.get());
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
                            <span class="status-bar__divider"></span>
                            <span class="status-bar__item">
                                {move || format_join_ms(join_round_trip_ms())}
                            </span>
                            <span class="status-bar__divider"></span>
                            <span class="status-bar__item">
                                {move || format_render_ms(render_ms())}
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
fn format_join_ms(ms: Option<f64>) -> String {
    match ms {
        Some(value) => format!("join {}ms", value.round() as i64),
        None => "join --ms".to_owned(),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn format_render_ms(ms: Option<f64>) -> String {
    match ms {
        Some(value) => format!("render {}ms", value.round() as i64),
        None => "render --ms".to_owned(),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn round_coord(value: f64) -> i64 {
    value.round() as i64
}

fn connection_status_class(status: ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Connected => "status-bar__dot status-bar__dot--connected",
        ConnectionStatus::Connecting => "status-bar__dot status-bar__dot--connecting",
        ConnectionStatus::Disconnected => "status-bar__dot status-bar__dot--disconnected",
    }
}

fn board_object_count(board: &BoardState) -> usize {
    board.objects.len()
}

fn board_join_ms(board: &BoardState) -> Option<f64> {
    board.join_round_trip_ms
}

fn board_camera_locked(board: &BoardState) -> bool {
    board.follow_client_id.is_some()
}

fn canvas_cursor(view: &CanvasViewState) -> Option<Point> {
    view.cursor_world.clone()
}

fn canvas_center(view: &CanvasViewState) -> Point {
    view.camera_center_world.clone()
}

fn canvas_fps(view: &CanvasViewState) -> Option<f64> {
    view.fps
}

fn canvas_render_ms(view: &CanvasViewState) -> Option<f64> {
    view.last_render_ms
}

fn ui_is_trace_mode(ui: &UiState) -> bool {
    ui.view_mode == ViewMode::Trace
}

fn trace_frame_total(trace: &TraceState) -> usize {
    trace.total_frames()
}

fn trace_filter_display(trace: &TraceState) -> String {
    let prefixes = trace.filter.active_prefixes();
    if prefixes.len() >= 5 {
        "ALL".to_owned()
    } else {
        prefixes.join("+")
    }
}

fn trace_mode_display(trace: &TraceState) -> &'static str {
    if trace.paused { "PAUSED" } else { "LIVE ●" }
}
