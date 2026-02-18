use crate::net::types::Point;

/// Live canvas telemetry consumed by chrome (status bar).
#[derive(Clone, Debug)]
pub struct CanvasViewState {
    pub cursor_world: Option<Point>,
    pub viewport_center_world: Point,
    pub zoom: f64,
}

impl Default for CanvasViewState {
    fn default() -> Self {
        Self { cursor_world: None, viewport_center_world: Point { x: 0.0, y: 0.0 }, zoom: 1.0 }
    }
}
