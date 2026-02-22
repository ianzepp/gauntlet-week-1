//! Bottom status bar showing connection status and canvas telemetry.
//!
//! SYSTEM CONTEXT
//! ==============
//! Renders low-frequency board telemetry so users can monitor connection and
//! viewport state without opening additional panels.

use leptos::prelude::*;
use leptos::tachys::view::any_view::IntoAny;

use crate::net::types::Point;
use crate::state::board::{BoardState, ConnectionStatus};
use crate::state::canvas_view::CanvasViewState;
use crate::state::trace::TraceState;
use crate::state::ui::{UiState, ViewMode};
use crate::util::animation::resolve_active_clip;

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
    let clip_summary = move || {
        let board_state = board.get();
        let ui_state = ui.get();
        resolve_active_clip(&board_state, &ui_state).map(|(id, clip)| (id, clip.duration_ms))
    };
    let clip_playhead = move || ui.get().animation_playhead_ms.max(0.0);
    let clip_is_playing = move || ui.get().animation_playing;

    let on_anim_bind = move |_| {
        let selected = board.get().selection.iter().next().cloned();
        if let Some(id) = selected {
            ui.update(|u| {
                u.animation_clip_object_id = Some(id);
                u.animation_playhead_ms = 0.0;
                u.animation_playing = false;
            });
        }
    };
    let on_anim_play_pause = move |_| {
        let board_state = board.get();
        let ui_state = ui.get();
        if let Some((id, clip)) = resolve_active_clip(&board_state, &ui_state) {
            ui.update(|u| {
                if u.animation_clip_object_id.as_deref() != Some(id.as_str()) {
                    u.animation_clip_object_id = Some(id.clone());
                    u.animation_playhead_ms = 0.0;
                }
                if u.animation_playhead_ms >= clip.duration_ms {
                    u.animation_playhead_ms = 0.0;
                }
                u.animation_playing = !u.animation_playing;
            });
        }
    };
    let on_anim_back = move |_| {
        let step = ui.get().animation_scrub_step_ms.max(1.0);
        ui.update(|u| {
            u.animation_playing = false;
            u.animation_playhead_ms = (u.animation_playhead_ms - step).max(0.0);
        });
    };
    let on_anim_fwd = move |_| {
        let board_state = board.get();
        let ui_state = ui.get();
        let duration = resolve_active_clip(&board_state, &ui_state)
            .map(|(_, clip)| clip.duration_ms)
            .unwrap_or(0.0);
        let step = ui_state.animation_scrub_step_ms.max(1.0);
        ui.update(|u| {
            u.animation_playing = false;
            u.animation_playhead_ms = (u.animation_playhead_ms + step).min(duration.max(0.0));
        });
    };

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
                            {move || {
                                if clip_summary().is_some() {
                                    view! {
                                        <>
                                            <span class="status-bar__divider"></span>
                                            <button class="status-bar__help status-bar__control" on:click=on_anim_bind title="Bind selected object as active animation clip">
                                                "[ANIM]"
                                            </button>
                                            <button class="status-bar__help status-bar__control" on:click=on_anim_back title="Step backward">
                                                "[<<]"
                                            </button>
                                            <button class="status-bar__help status-bar__control" on:click=on_anim_play_pause title="Play or pause animation">
                                                {move || if clip_is_playing() { "[PAUSE]" } else { "[PLAY]" }}
                                            </button>
                                            <button class="status-bar__help status-bar__control" on:click=on_anim_fwd title="Step forward">
                                                "[>>]"
                                            </button>
                                            <span class="status-bar__item">
                                                {move || {
                                                    if let Some((_id, duration_ms)) = clip_summary() {
                                                        format_anim_time(clip_playhead(), duration_ms)
                                                    } else {
                                                        "anim --/--".to_owned()
                                                    }
                                                }}
                                            </span>
                                        </>
                                    }
                                    .into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
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
fn format_anim_time(playhead_ms: f64, duration_ms: f64) -> String {
    let playhead = playhead_ms.round() as i64;
    let duration = duration_ms.round() as i64;
    format!("anim {playhead}/{duration}ms")
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
