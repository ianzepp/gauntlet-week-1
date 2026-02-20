//! Canvas viewport/camera synchronization helpers shared by canvas host.

#[cfg(feature = "hydrate")]
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use crate::app::FrameSender;
#[cfg(feature = "hydrate")]
use crate::net::types::{Frame, FrameStatus, Point as WirePoint};
#[cfg(feature = "hydrate")]
use crate::state::auth::AuthState;
#[cfg(feature = "hydrate")]
use crate::state::board::{BoardState, ConnectionStatus};
#[cfg(feature = "hydrate")]
use crate::state::canvas_view::CanvasViewState;

#[cfg(feature = "hydrate")]
use canvas::camera::Point as CanvasPoint;
#[cfg(feature = "hydrate")]
use canvas::engine::Engine;

#[cfg(feature = "hydrate")]
pub fn sync_viewport(engine: &mut Engine, canvas_ref: &NodeRef<leptos::html::Canvas>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(canvas) = canvas_ref.get() else {
        return;
    };
    let width = f64::from(canvas.client_width()).max(1.0);
    let height = f64::from(canvas.client_height()).max(1.0);
    let dpr = window.device_pixel_ratio().max(1.0);
    engine.set_viewport(width, height, dpr);
}

#[cfg(feature = "hydrate")]
pub fn center_world_origin(engine: &mut Engine) {
    engine.core.camera.pan_x = engine.core.viewport_width * 0.5;
    engine.core.camera.pan_y = engine.core.viewport_height * 0.5;
}

#[cfg(feature = "hydrate")]
pub fn set_camera_view(engine: &mut Engine, center_x: f64, center_y: f64, zoom: f64, rotation_deg: f64) {
    let clamped_zoom = zoom.clamp(0.1, 10.0);
    engine.core.camera.zoom = clamped_zoom;
    engine.set_view_rotation_deg(rotation_deg);
    engine.core.camera.pan_x = (engine.core.viewport_width * 0.5) - (center_x * clamped_zoom);
    engine.core.camera.pan_y = (engine.core.viewport_height * 0.5) - (center_y * clamped_zoom);
}

#[cfg(feature = "hydrate")]
pub fn zoom_view_preserving_center(engine: &mut Engine, zoom: f64) {
    let center_screen = viewport_center_screen(engine);
    let center_world = engine
        .camera()
        .screen_to_world(center_screen, center_screen);
    let rotation = engine.view_rotation_deg();
    set_camera_view(engine, center_world.x, center_world.y, zoom, rotation);
}

#[cfg(feature = "hydrate")]
pub fn viewport_center_screen(engine: &Engine) -> CanvasPoint {
    CanvasPoint::new(engine.core.viewport_width * 0.5, engine.core.viewport_height * 0.5)
}

#[cfg(feature = "hydrate")]
pub fn send_cursor_presence_if_needed(
    engine: &Engine,
    board: RwSignal<BoardState>,
    auth: RwSignal<AuthState>,
    sender: RwSignal<FrameSender>,
    last_sent_ms: RwSignal<f64>,
    last_sent_view: RwSignal<Option<(f64, f64, f64, f64)>>,
    cursor_screen: Option<CanvasPoint>,
    force: bool,
) {
    let state = board.get_untracked();
    if state.connection_status != ConnectionStatus::Connected {
        return;
    }
    if state.self_client_id.is_none() {
        return;
    }
    let Some(user) = auth.get_untracked().user else {
        return;
    };
    let has_cursor_point = cursor_screen.is_some();
    const CAMERA_ONLY_MIN_INTERVAL_MS: f64 = 40.0;
    const CAMERA_CENTER_DEADBAND_WORLD: f64 = 0.2;
    const CAMERA_ZOOM_DEADBAND: f64 = 0.001;
    const CAMERA_ROTATION_DEADBAND_DEG: f64 = 0.1;

    let now = now_ms();
    if !force && !has_cursor_point && now - last_sent_ms.get_untracked() < CAMERA_ONLY_MIN_INTERVAL_MS {
        return;
    }
    let Some(board_id) = state.board_id else {
        return;
    };

    let camera = engine.camera();
    let center_screen = viewport_center_screen(engine);
    let center_world = camera.screen_to_world(center_screen, center_screen);
    let center_x = center_world.x;
    let center_y = center_world.y;
    let zoom = camera.zoom;
    let rotation = camera.view_rotation_deg;
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p, center_screen));

    if !force
        && !has_cursor_point
        && let Some((last_x, last_y, last_zoom, last_rotation)) = last_sent_view.get_untracked()
    {
        let dx = center_x - last_x;
        let dy = center_y - last_y;
        let center_dist = (dx * dx + dy * dy).sqrt();
        let zoom_delta = (zoom - last_zoom).abs();
        let rotation_delta = (rotation - last_rotation).abs();
        if center_dist < CAMERA_CENTER_DEADBAND_WORLD
            && zoom_delta < CAMERA_ZOOM_DEADBAND
            && rotation_delta < CAMERA_ROTATION_DEADBAND_DEG
        {
            return;
        }
    }

    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "cursor:moved".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "x": cursor_world.as_ref().map(|p| p.x),
            "y": cursor_world.as_ref().map(|p| p.y),
            "name": user.name,
            "color": user.color,
            "camera_center_x": center_x,
            "camera_center_y": center_y,
            "camera_zoom": zoom,
            "camera_rotation": rotation,
        }),
    };
    if sender.get_untracked().send(&frame) {
        last_sent_ms.set(now);
        last_sent_view.set(Some((center_x, center_y, zoom, rotation)));
    }
}

#[cfg(feature = "hydrate")]
pub fn sync_canvas_view_state(
    engine: &Engine,
    canvas_view: RwSignal<CanvasViewState>,
    cursor_screen: Option<CanvasPoint>,
) {
    let camera = engine.camera();
    let camera_center_screen = viewport_center_screen(engine);
    let camera_center_world = camera.screen_to_world(camera_center_screen, camera_center_screen);
    let cursor_world = cursor_screen.map(|p| camera.screen_to_world(p, camera_center_screen));
    let sample_ms = now_ms();

    canvas_view.update(|v| {
        if let Some(prev_sample_ms) = v.fps_last_sample_ms {
            let dt_ms = sample_ms - prev_sample_ms;
            if (5.0..2000.0).contains(&dt_ms) {
                let instantaneous_fps = 1000.0 / dt_ms;
                let ema_alpha = 0.2;
                v.fps = Some(v.fps.map_or(instantaneous_fps, |prev| {
                    (prev * (1.0 - ema_alpha)) + (instantaneous_fps * ema_alpha)
                }));
            }
        }
        v.fps_last_sample_ms = Some(sample_ms);
        v.cursor_world = cursor_world.map(|p| WirePoint { x: p.x, y: p.y });
        v.camera_center_world = WirePoint { x: camera_center_world.x, y: camera_center_world.y };
        v.zoom = camera.zoom;
        v.pan_x = camera.pan_x;
        v.pan_y = camera.pan_y;
        v.view_rotation_deg = camera.view_rotation_deg;
        v.viewport_width = engine.core.viewport_width;
        v.viewport_height = engine.core.viewport_height;
    });
}

#[cfg(feature = "hydrate")]
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}
