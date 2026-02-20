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

const ZOOM_PRESETS: [f64; 11] = [0.10, 0.25, 0.50, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 3.0, 4.0];

/// Status bar at the bottom of the board page.
#[component]
pub fn StatusBar() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let canvas_view = expect_context::<RwSignal<CanvasViewState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let trace = expect_context::<RwSignal<TraceState>>();
    let zoom_menu_open = RwSignal::new(false);

    let status_class = move || {
        let status = board.get().connection_status;
        match status {
            ConnectionStatus::Connected => "status-bar__dot status-bar__dot--connected",
            ConnectionStatus::Connecting => "status-bar__dot status-bar__dot--connecting",
            ConnectionStatus::Disconnected => "status-bar__dot status-bar__dot--disconnected",
        }
    };

    let object_count = move || board.get().objects.len();
    let board_name = move || board.get().board_name.unwrap_or_default();
    let camera_locked = move || board.get().follow_client_id.is_some();
    let cursor = move || canvas_view.get().cursor_world;
    let camera_center = move || canvas_view.get().camera_center_world.clone();
    let zoom = move || canvas_view.get().zoom;
    let fps = move || canvas_view.get().fps;
    let zoom_label = move || format_zoom(zoom());

    let set_zoom = move |target_zoom: f64| {
        ui.update(|u| {
            u.zoom_override = Some(target_zoom.clamp(0.1, 4.0));
            u.zoom_override_seq = u.zoom_override_seq.saturating_add(1);
        });
        zoom_menu_open.set(false);
    };

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

    view! {
        <div class="status-bar">
            // Left section: always show connection dot + board name
            <div class="status-bar__section">
                <span class="status-bar__item">
                    <span class=status_class></span>
                </span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__board-name">{board_name}</span>

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

                    <div class="status-bar__zoom">
                        <button
                            class="status-bar__zoom-button"
                            on:click=move |_| zoom_menu_open.set(!zoom_menu_open.get())
                            title="Set zoom"
                        >
                            {zoom_label}
                        </button>
                        <Show when=move || zoom_menu_open.get()>
                            <div class="status-bar__zoom-menu">
                                {ZOOM_PRESETS
                                    .into_iter()
                                    .map(|preset| {
                                        let is_active = move || (zoom() - preset).abs() < 0.005;
                                        view! {
                                            <button
                                                class="status-bar__zoom-option"
                                                class:status-bar__zoom-option--active=is_active
                                                on:click=move |_| set_zoom(preset)
                                            >
                                                {format_zoom(preset)}
                                            </button>
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    }
}

fn format_cursor(point: Option<Point>) -> String {
    point
        .map(format_point)
        .unwrap_or_else(|| "(-, -)".to_owned())
}

fn format_point(point: Point) -> String {
    format!("({}, {})", round_coord(point.x), round_coord(point.y))
}

fn format_zoom(zoom: f64) -> String {
    format!("{}%", (zoom * 100.0).round() as i64)
}

fn format_fps(fps: Option<f64>) -> String {
    match fps {
        Some(value) => format!("{} fps", value.round() as i64),
        None => "-- fps".to_owned(),
    }
}

fn round_coord(value: f64) -> i64 {
    value.round() as i64
}
