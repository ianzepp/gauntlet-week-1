//! Shared numeric constants for the canvas crate.

// ── Math ────────────────────────────────────────────────────────

/// π / 5 (36°) — angular step for a 10-vertex star polygon.
pub const FRAC_PI_5: f64 = std::f64::consts::PI / 5.0;

// ── Hit-testing ─────────────────────────────────────────────────

/// Screen-space hit slop in pixels for handles and thin edges.
pub const HANDLE_RADIUS_PX: f64 = 8.0;

/// Distance from the bounding box edge to the rotate handle, in screen pixels.
pub const ROTATE_HANDLE_OFFSET_PX: f64 = 24.0;

/// Inner-to-outer radius ratio for the default 5-point star.
pub const STAR_INNER_RATIO: f64 = 0.5;

// ── Input / gestures ──────────────────────────────────────────

/// Minimum dimension (world units) to keep a drawn shape on pointer-up.
/// Shapes smaller than this are treated as accidental clicks and discarded.
pub const MIN_SHAPE_SIZE: f64 = 2.0;

/// Zoom multiplier per wheel tick (scroll-up zooms in by this factor).
pub const ZOOM_FACTOR: f64 = 1.1;

/// Minimum allowed zoom level.
pub const ZOOM_MIN: f64 = 0.1;

/// Maximum allowed zoom level.
pub const ZOOM_MAX: f64 = 10.0;
