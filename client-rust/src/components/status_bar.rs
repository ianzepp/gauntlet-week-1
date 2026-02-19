//! Bottom status bar showing connection status, zoom level, and object count.

use leptos::prelude::*;

use crate::net::types::Point;
use crate::state::auth::AuthState;
use crate::state::board::{BoardState, ConnectionStatus};
use crate::state::canvas_view::CanvasViewState;

/// Status bar at the bottom of the board page.
#[component]
pub fn StatusBar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let canvas_view = expect_context::<RwSignal<CanvasViewState>>();

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

    let user = move || auth.get().user;

    view! {
        <div class="status-bar">
            <div class="status-bar__section">
                <span class="status-bar__item">
                    <span class=status_class></span>
                </span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__board-name">{board_name}</span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__item">{move || format!("{} objs", object_count())}</span>

                <Show when=camera_locked>
                    <span class="status-bar__divider"></span>
                    <span class="status-bar__item">"LOCKED CAMERA"</span>
                </Show>
            </div>

            <div class="status-bar__section">
                <span class="status-bar__item">{move || format_cursor(cursor())}</span>

                <span class="status-bar__divider"></span>
                <span class="status-bar__item">{move || format_point(camera_center())}</span>

                <span class="status-bar__divider"></span>
                <Show when=move || user().is_some()>
                    <span class="status-bar__user-chip">
                        <span class="status-bar__user-dot" style:background=move || user().map_or_else(String::new, |u| u.color)></span>
                        {move || user().map_or_else(String::new, |u| u.name)}
                    </span>
                    <span class="status-bar__divider"></span>
                </Show>

                <span class="status-bar__item">{move || format_zoom(zoom())}</span>
            </div>
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

fn round_coord(value: f64) -> i64 {
    value.round() as i64
}
