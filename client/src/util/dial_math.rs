//! Shared dial/angle math for canvas controls.
//!
//! Canvas dials map a continuous value (zoom, border width, font size, lightness) onto a
//! ±135° arc. This module centralises the bidirectional mapping formulas, the tick-tension
//! snapping algorithm, and the display formatters so that all dials behave consistently.

#[cfg(test)]
#[path = "dial_math_test.rs"]
mod dial_math_test;

/// The minimum angle (degrees) of any dial arc — the hard left stop.
pub const ZOOM_DIAL_MIN_ANGLE_DEG: f64 = -135.0;

/// The maximum angle (degrees) of any dial arc — the hard right stop.
pub const ZOOM_DIAL_MAX_ANGLE_DEG: f64 = 135.0;

/// The angular half-width (degrees) around each tick within which tension attraction begins.
///
/// A pointer inside this band is pulled toward the nearest tick, creating a tactile "notch".
pub const ZOOM_DIAL_TICK_TENSION_RANGE_DEG: f64 = 14.0;

/// The fraction of the remaining distance to a tick that is added per tick-tension step.
///
/// Larger values produce stronger notch attraction; 0.42 gives a perceptible but not
/// overpowering pull.
pub const ZOOM_DIAL_TICK_TENSION_STRENGTH: f64 = 0.42;

/// Minimum allowed border stroke width in pixels.
pub const BORDER_WIDTH_MIN: f64 = 0.0;

/// Maximum allowed border stroke width in pixels.
pub const BORDER_WIDTH_MAX: f64 = 24.0;

/// Minimum allowed font size in pixels.
pub const TEXT_SIZE_MIN: f64 = 8.0;

/// Maximum allowed font size in pixels.
pub const TEXT_SIZE_MAX: f64 = 96.0;

/// Normalise an angle to the half-open range [0°, 360°).
pub fn normalize_degrees_360(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

/// Compute the shortest signed angular distance from `start` to `current`, in degrees.
///
/// The result is in (-180°, +180°]. Positive means clockwise, negative means
/// counter-clockwise. Returns 0 for non-finite inputs to avoid NaN propagation.
/// Use this when you need to know *which direction* the dial moved from its starting position.
pub fn signed_angle_delta_deg(current: f64, start: f64) -> f64 {
    let delta = current - start;
    if !delta.is_finite() {
        return 0.0;
    }
    let mut wrapped = delta.rem_euclid(360.0);
    if wrapped > 180.0 {
        wrapped -= 360.0;
    }
    wrapped
}

/// Compute the absolute (unsigned) shortest angular distance between two angles, in degrees.
///
/// Always returns a value in [0°, 180°]. Use this when you only need to know how far apart
/// two angles are, not which direction, for example when checking a deadband threshold.
pub fn angular_delta_deg(a: f64, b: f64) -> f64 {
    let delta = (a - b).abs().rem_euclid(360.0);
    delta.min(360.0 - delta)
}

/// Apply cardinal-direction snapping (and optional 15° step snapping) to a compass bearing.
///
/// Snaps to the nearest cardinal (0°, 90°, 180°, 270°) when within 6° of it, providing a
/// tactile lock at the four primary orientations. If `shift_snap` is true, the angle is
/// additionally quantised to the nearest 15° step, useful for keyboard-modified rotation.
/// Returns a normalised angle in [0°, 360°).
pub fn apply_compass_drag_snapping(raw_deg: f64, shift_snap: bool) -> f64 {
    const CARDINAL_SNAP_EPS_DEG: f64 = 6.0;
    const SHIFT_STEP_DEG: f64 = 15.0;

    let mut deg = normalize_degrees_360(raw_deg);
    for target in [0.0, 90.0, 180.0, 270.0] {
        if angular_delta_deg(deg, target) <= CARDINAL_SNAP_EPS_DEG {
            deg = target;
            break;
        }
    }
    if shift_snap {
        deg = (deg / SHIFT_STEP_DEG).round() * SHIFT_STEP_DEG;
    }
    normalize_degrees_360(deg)
}

/// Apply tick-tension attraction to a zoom dial angle.
///
/// The zoom dial has seven evenly-spaced tick positions. When the pointer angle falls within
/// [`ZOOM_DIAL_TICK_TENSION_RANGE_DEG`] of a tick, the angle is pulled toward that tick by
/// `distance_to_tick * weight * ZOOM_DIAL_TICK_TENSION_STRENGTH`. This creates a soft magnetic
/// notch without hard-locking the dial. The result is clamped to the dial arc.
pub fn apply_zoom_tick_tension(angle: f64) -> f64 {
    let ticks = [
        ZOOM_DIAL_MIN_ANGLE_DEG,
        -90.0,
        -45.0,
        0.0,
        45.0,
        90.0,
        ZOOM_DIAL_MAX_ANGLE_DEG,
    ];
    let mut adjusted = angle;
    for tick in ticks {
        let distance = (adjusted - tick).abs();
        if distance >= ZOOM_DIAL_TICK_TENSION_RANGE_DEG {
            continue;
        }
        let weight = 1.0 - (distance / ZOOM_DIAL_TICK_TENSION_RANGE_DEG);
        adjusted += (tick - adjusted) * weight * ZOOM_DIAL_TICK_TENSION_STRENGTH;
    }
    adjusted.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

/// Map a zoom multiplier to a dial angle using a linear formula.
///
/// The mapping is `angle = (zoom - 1) * 180`, placing zoom 1.0 at 0° (centre),
/// zoom 0.25 at -135° (min stop), and zoom 1.75 at +135° (max stop).
pub fn dial_angle_from_zoom(zoom: f64) -> f64 {
    ((zoom - 1.0) * 180.0).clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

/// Map a dial angle back to a zoom multiplier, inverse of [`dial_angle_from_zoom`].
///
/// Returns a value clamped to [0.1, 10.0] to prevent zero or negative zoom.
pub fn zoom_from_dial_angle(angle: f64) -> f64 {
    (1.0 + (angle / 180.0)).clamp(0.1, 10.0)
}

/// Map a lightness shift value in [-1, 1] to a dial angle.
///
/// The full [-1, 1] range maps linearly onto [MIN_ANGLE, MAX_ANGLE], placing 0 (no shift)
/// at 0° and extremes at the dial stops.
pub fn dial_angle_from_color_shift(shift: f64) -> f64 {
    (shift.clamp(-1.0, 1.0) * ZOOM_DIAL_MAX_ANGLE_DEG).clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

/// Map a dial angle back to a lightness shift, inverse of [`dial_angle_from_color_shift`].
///
/// Returns a value clamped to [-1, 1].
pub fn color_shift_from_dial_angle(angle: f64) -> f64 {
    (angle / ZOOM_DIAL_MAX_ANGLE_DEG).clamp(-1.0, 1.0)
}

/// Map a border width in pixels to a dial angle using a linear range mapping.
///
/// [`BORDER_WIDTH_MIN`] maps to [`ZOOM_DIAL_MIN_ANGLE_DEG`] and [`BORDER_WIDTH_MAX`] to
/// [`ZOOM_DIAL_MAX_ANGLE_DEG`]. Handles the degenerate case where min == max by returning 0°.
pub fn dial_angle_from_border_width(width: f64) -> f64 {
    let clamped = width.clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    let t = if BORDER_WIDTH_MAX <= BORDER_WIDTH_MIN {
        0.0
    } else {
        (clamped - BORDER_WIDTH_MIN) / (BORDER_WIDTH_MAX - BORDER_WIDTH_MIN)
    };
    ZOOM_DIAL_MIN_ANGLE_DEG + (t * (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG))
}

/// Map a dial angle back to a border width in pixels, inverse of [`dial_angle_from_border_width`].
///
/// The result is snapped to the nearest integer pixel before returning.
pub fn border_width_from_dial_angle(angle: f64) -> f64 {
    let clamped_angle = angle.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    let t = (clamped_angle - ZOOM_DIAL_MIN_ANGLE_DEG) / (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG);
    snap_border_width_to_px(BORDER_WIDTH_MIN + (t * (BORDER_WIDTH_MAX - BORDER_WIDTH_MIN)))
}

/// Map a font size in pixels to a dial angle using a linear range mapping.
///
/// [`TEXT_SIZE_MIN`] maps to [`ZOOM_DIAL_MIN_ANGLE_DEG`] and [`TEXT_SIZE_MAX`] to
/// [`ZOOM_DIAL_MAX_ANGLE_DEG`]. Handles the degenerate case where min == max by returning 0°.
pub fn dial_angle_from_font_size(size: f64) -> f64 {
    let clamped = size.clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX);
    let t = if TEXT_SIZE_MAX <= TEXT_SIZE_MIN {
        0.0
    } else {
        (clamped - TEXT_SIZE_MIN) / (TEXT_SIZE_MAX - TEXT_SIZE_MIN)
    };
    ZOOM_DIAL_MIN_ANGLE_DEG + (t * (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG))
}

/// Map a dial angle back to a font size in pixels, inverse of [`dial_angle_from_font_size`].
///
/// The result is snapped to the nearest integer pixel before returning.
pub fn font_size_from_dial_angle(angle: f64) -> f64 {
    let clamped_angle = angle.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    let t = (clamped_angle - ZOOM_DIAL_MIN_ANGLE_DEG) / (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG);
    snap_font_size_to_px(TEXT_SIZE_MIN + (t * (TEXT_SIZE_MAX - TEXT_SIZE_MIN)))
}

/// Format a border width as a human-readable pixel label.
///
/// Emits an integer label (e.g. `"2px"`) when the value rounds cleanly, otherwise one decimal
/// place (e.g. `"1.5px"`).
pub fn format_border_width_label(width: f64) -> String {
    let rounded = width.round();
    if (width - rounded).abs() < 0.05 {
        format!("{}px", rounded as i64)
    } else {
        format!("{width:.1}px")
    }
}

/// Round a border width to the nearest integer pixel and clamp to the valid range.
pub fn snap_border_width_to_px(width: f64) -> f64 {
    width.round().clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

/// Format a font size as a human-readable pixel label using the snapped integer value.
pub fn format_text_size_label(size: f64) -> String {
    format!("{}px", snap_font_size_to_px(size) as i64)
}

/// Round a font size to the nearest integer pixel and clamp to the valid range.
pub fn snap_font_size_to_px(size: f64) -> f64 {
    size.round().clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX)
}
