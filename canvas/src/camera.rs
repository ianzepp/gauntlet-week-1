#[cfg(test)]
#[path = "camera_test.rs"]
mod camera_test;

/// A point in either screen or world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Camera state for pan/zoom on the infinite canvas.
///
/// `pan_x` / `pan_y` are in CSS pixels.
/// `zoom` is a scale factor (1.0 = no zoom).
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self { pan_x: 0.0, pan_y: 0.0, zoom: 1.0 }
    }
}

impl Camera {
    /// Convert a screen-space point (CSS pixels) to world coordinates.
    #[must_use]
    pub fn screen_to_world(&self, screen: Point) -> Point {
        Point {
            x: (screen.x - self.pan_x) / self.zoom,
            y: (screen.y - self.pan_y) / self.zoom,
        }
    }

    /// Convert a world-space point to screen coordinates (CSS pixels).
    #[must_use]
    pub fn world_to_screen(&self, world: Point) -> Point {
        Point {
            x: world.x * self.zoom + self.pan_x,
            y: world.y * self.zoom + self.pan_y,
        }
    }

    /// Convert a screen-space distance (pixels) to world-space distance.
    #[must_use]
    pub fn screen_dist_to_world(&self, screen_dist: f64) -> f64 {
        screen_dist / self.zoom
    }
}
