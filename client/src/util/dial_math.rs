//! Shared dial/angle math for canvas controls.

#[cfg(test)]
#[path = "dial_math_test.rs"]
mod dial_math_test;

pub const ZOOM_DIAL_MIN_ANGLE_DEG: f64 = -135.0;
pub const ZOOM_DIAL_MAX_ANGLE_DEG: f64 = 135.0;
pub const ZOOM_DIAL_TICK_TENSION_RANGE_DEG: f64 = 14.0;
pub const ZOOM_DIAL_TICK_TENSION_STRENGTH: f64 = 0.42;

pub const BORDER_WIDTH_MIN: f64 = 0.0;
pub const BORDER_WIDTH_MAX: f64 = 24.0;
pub const TEXT_SIZE_MIN: f64 = 8.0;
pub const TEXT_SIZE_MAX: f64 = 96.0;

pub fn normalize_degrees_360(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

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

pub fn angular_delta_deg(a: f64, b: f64) -> f64 {
    let delta = (a - b).abs().rem_euclid(360.0);
    delta.min(360.0 - delta)
}

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

pub fn dial_angle_from_zoom(zoom: f64) -> f64 {
    ((zoom - 1.0) * 180.0).clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

pub fn zoom_from_dial_angle(angle: f64) -> f64 {
    (1.0 + (angle / 180.0)).clamp(0.1, 10.0)
}

pub fn dial_angle_from_color_shift(shift: f64) -> f64 {
    (shift.clamp(-1.0, 1.0) * ZOOM_DIAL_MAX_ANGLE_DEG).clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG)
}

pub fn color_shift_from_dial_angle(angle: f64) -> f64 {
    (angle / ZOOM_DIAL_MAX_ANGLE_DEG).clamp(-1.0, 1.0)
}

pub fn dial_angle_from_border_width(width: f64) -> f64 {
    let clamped = width.clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    let t = if BORDER_WIDTH_MAX <= BORDER_WIDTH_MIN {
        0.0
    } else {
        (clamped - BORDER_WIDTH_MIN) / (BORDER_WIDTH_MAX - BORDER_WIDTH_MIN)
    };
    ZOOM_DIAL_MIN_ANGLE_DEG + (t * (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG))
}

pub fn border_width_from_dial_angle(angle: f64) -> f64 {
    let clamped_angle = angle.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    let t = (clamped_angle - ZOOM_DIAL_MIN_ANGLE_DEG) / (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG);
    snap_border_width_to_px(BORDER_WIDTH_MIN + (t * (BORDER_WIDTH_MAX - BORDER_WIDTH_MIN)))
}

pub fn dial_angle_from_font_size(size: f64) -> f64 {
    let clamped = size.clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX);
    let t = if TEXT_SIZE_MAX <= TEXT_SIZE_MIN {
        0.0
    } else {
        (clamped - TEXT_SIZE_MIN) / (TEXT_SIZE_MAX - TEXT_SIZE_MIN)
    };
    ZOOM_DIAL_MIN_ANGLE_DEG + (t * (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG))
}

pub fn font_size_from_dial_angle(angle: f64) -> f64 {
    let clamped_angle = angle.clamp(ZOOM_DIAL_MIN_ANGLE_DEG, ZOOM_DIAL_MAX_ANGLE_DEG);
    let t = (clamped_angle - ZOOM_DIAL_MIN_ANGLE_DEG) / (ZOOM_DIAL_MAX_ANGLE_DEG - ZOOM_DIAL_MIN_ANGLE_DEG);
    snap_font_size_to_px(TEXT_SIZE_MIN + (t * (TEXT_SIZE_MAX - TEXT_SIZE_MIN)))
}

pub fn format_border_width_label(width: f64) -> String {
    let rounded = width.round();
    if (width - rounded).abs() < 0.05 {
        format!("{}px", rounded as i64)
    } else {
        format!("{width:.1}px")
    }
}

pub fn snap_border_width_to_px(width: f64) -> f64 {
    width.round().clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

pub fn format_text_size_label(size: f64) -> String {
    format!("{}px", snap_font_size_to_px(size) as i64)
}

pub fn snap_font_size_to_px(size: f64) -> f64 {
    size.round().clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX)
}
