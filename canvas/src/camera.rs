//! Camera model for the infinite canvas: pan, zoom, and coordinate conversion.
//!
//! All canvas state is stored in world coordinates. The [`Camera`] converts
//! between world space and screen (CSS pixel) space when rendering or
//! interpreting pointer events.

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
    /// Create a new point with the given coordinates.
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
    pub view_rotation_deg: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self { pan_x: 0.0, pan_y: 0.0, zoom: 1.0, view_rotation_deg: 0.0 }
    }
}

impl Camera {
    /// Convert a screen-space point (CSS pixels) to world coordinates.
    #[must_use]
    pub fn screen_to_world(&self, screen: Point, viewport_center: Point) -> Point {
        let unrotated = rotate_point(screen, viewport_center, -self.view_rotation_deg);
        Point { x: (unrotated.x - self.pan_x) / self.zoom, y: (unrotated.y - self.pan_y) / self.zoom }
    }

    /// Convert a world-space point to screen coordinates (CSS pixels).
    #[must_use]
    pub fn world_to_screen(&self, world: Point, viewport_center: Point) -> Point {
        let unrotated = Point { x: world.x * self.zoom + self.pan_x, y: world.y * self.zoom + self.pan_y };
        rotate_point(unrotated, viewport_center, self.view_rotation_deg)
    }

    /// Convert a screen-space distance (pixels) to world-space distance.
    #[must_use]
    pub fn screen_dist_to_world(&self, screen_dist: f64) -> f64 {
        screen_dist / self.zoom
    }
}

fn rotate_point(point: Point, center: Point, deg: f64) -> Point {
    let rad = deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point { x: center.x + (dx * cos) - (dy * sin), y: center.y + (dx * sin) + (dy * cos) }
}
