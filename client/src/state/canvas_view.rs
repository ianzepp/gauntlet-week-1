//! Canvas viewport telemetry used by non-canvas UI surfaces.
//!
//! ARCHITECTURE
//! ============
//! `CanvasHost` owns authoritative camera/cursor coordinates and publishes
//! snapshots through this struct so other components (status bar, overlays)
//! can render without direct canvas coupling.

#[cfg(test)]
#[path = "canvas_view_test.rs"]
mod canvas_view_test;

use crate::net::types::Point;

/// Live canvas telemetry consumed by chrome (status bar).
#[derive(Clone, Debug)]
pub struct CanvasViewState {
    /// Current cursor position in world coordinates, or `None` when the cursor is outside the canvas.
    pub cursor_world: Option<Point>,
    /// Center of the viewport in world coordinates.
    pub camera_center_world: Point,
    /// Current zoom scale factor (1.0 = no zoom).
    pub zoom: f64,
    /// Most recently measured frames per second, or `None` before the first sample.
    pub fps: Option<f64>,
    /// Timestamp of the last FPS sample in milliseconds.
    pub fps_last_sample_ms: Option<f64>,
    /// Camera pan offset along the x-axis in CSS pixels.
    pub pan_x: f64,
    /// Camera pan offset along the y-axis in CSS pixels.
    pub pan_y: f64,
    /// Canvas view rotation in degrees.
    pub view_rotation_deg: f64,
    /// Viewport width in CSS pixels.
    pub viewport_width: f64,
    /// Viewport height in CSS pixels.
    pub viewport_height: f64,
}

impl Default for CanvasViewState {
    fn default() -> Self {
        Self {
            cursor_world: None,
            camera_center_world: Point { x: 0.0, y: 0.0 },
            zoom: 1.0,
            fps: None,
            fps_last_sample_ms: None,
            pan_x: 0.0,
            pan_y: 0.0,
            view_rotation_deg: 0.0,
            viewport_width: 0.0,
            viewport_height: 0.0,
        }
    }
}
