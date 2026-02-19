//! Canvas viewport telemetry used by non-canvas UI surfaces.
//!
//! ARCHITECTURE
//! ============
//! `CanvasHost` owns authoritative camera/cursor coordinates and publishes
//! snapshots through this struct so other components (status bar, overlays)
//! can render without direct canvas coupling.

use crate::net::types::Point;

/// Live canvas telemetry consumed by chrome (status bar).
#[derive(Clone, Debug)]
pub struct CanvasViewState {
    pub cursor_world: Option<Point>,
    pub camera_center_world: Point,
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
}

impl Default for CanvasViewState {
    fn default() -> Self {
        Self { cursor_world: None, camera_center_world: Point { x: 0.0, y: 0.0 }, zoom: 1.0, pan_x: 0.0, pan_y: 0.0 }
    }
}
