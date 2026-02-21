//! Selection aggregate helpers for board objects.
//!
//! Each function reduces the set of currently selected objects to a single representative value
//! that is displayed in the inspector panel. "Representative" follows a consistent strategy:
//! - For string values (colors) the first selected object wins — showing multiple colors at once
//!   would require a colour swatch UI that does not exist yet.
//! - For numeric values (widths, sizes, scale) the arithmetic mean is used so that a mixed
//!   selection shows something sensible in the middle of the range.
//! - For rotation the circular mean is used to avoid the wraparound discontinuity near 0°/360°.

#[cfg(test)]
#[path = "selection_metrics_test.rs"]
mod selection_metrics_test;

use crate::state::board::BoardState;
use crate::util::dial_math::{BORDER_WIDTH_MAX, BORDER_WIDTH_MIN, TEXT_SIZE_MAX, TEXT_SIZE_MIN, normalize_degrees_360};
use crate::util::object_props::{
    object_base_fill_hex, object_border_color_hex, object_border_width, object_font_size, object_lightness_shift,
    object_scale_components, object_text_color_hex,
};

/// Return the base fill color of the first selected object, or the default red `#D94B4B`.
///
/// "First" is determined by iteration order of the selection set, not z-order. Using the first
/// object rather than averaging avoids producing a blended color that matches no object.
pub fn representative_base_color_hex(state: &BoardState) -> String {
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_base_fill_hex))
        .unwrap_or_else(|| "#D94B4B".to_owned())
}

/// Return the arithmetic mean lightness shift across all selected objects, clamped to [-1, 1].
///
/// Returns 0.0 (no shift) when nothing is selected.
pub fn representative_lightness_shift(state: &BoardState) -> f64 {
    let mut shifts: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        shifts.push(object_lightness_shift(obj));
    }
    if shifts.is_empty() {
        return 0.0;
    }
    (shifts.iter().sum::<f64>() / shifts.len() as f64).clamp(-1.0, 1.0)
}

/// Return the border color of the first selected object, or the default charcoal `#1F1A17`.
pub fn representative_border_color_hex(state: &BoardState) -> String {
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_border_color_hex))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

/// Return the arithmetic mean border width across all selected objects.
///
/// The result is clamped to [`BORDER_WIDTH_MIN`]..=[`BORDER_WIDTH_MAX`].
/// Returns 1.0 as a neutral default when nothing is selected.
pub fn representative_border_width(state: &BoardState) -> f64 {
    let mut widths: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        widths.push(object_border_width(obj));
    }
    if widths.is_empty() {
        return 1.0;
    }
    (widths.iter().sum::<f64>() / widths.len() as f64).clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

/// Return the text color of the first selected object, or the default charcoal `#1F1A17`.
pub fn representative_text_color_hex(state: &BoardState) -> String {
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_text_color_hex))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

/// Return the arithmetic mean font size across all selected objects, rounded to the nearest pixel.
///
/// The result is clamped to [`TEXT_SIZE_MIN`]..=[`TEXT_SIZE_MAX`].
/// Returns 24.0 (the default body font size) when nothing is selected.
pub fn representative_font_size(state: &BoardState) -> f64 {
    let mut sizes: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        sizes.push(object_font_size(obj));
    }
    if sizes.is_empty() {
        return 24.0;
    }
    ((sizes.iter().sum::<f64>() / sizes.len() as f64).round()).clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX)
}

/// Return the circular mean rotation of all selected objects in degrees, normalised to [0°, 360°).
///
/// A simple arithmetic mean of angles fails near the 0°/360° boundary (e.g. the mean of 10°
/// and 350° would incorrectly compute to 180° instead of 0°). The circular mean projects each
/// angle onto the unit circle as `(cos θ, sin θ)`, sums the vectors, and takes the atan2 of
/// the resultant — correctly handling wraparound. Returns 0.0 when nothing is selected.
pub fn representative_rotation_deg(state: &BoardState) -> f64 {
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut count = 0_usize;
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        let r = obj.rotation.to_radians();
        sum_x += r.cos();
        sum_y += r.sin();
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    normalize_degrees_360(sum_y.atan2(sum_x).to_degrees())
}

/// Return the arithmetic mean scale factor across all selected objects.
///
/// Returns 1.0 (no scaling) when nothing is selected.
pub fn representative_scale_factor(state: &BoardState) -> f64 {
    let mut scales: Vec<f64> = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        let width = obj.width.unwrap_or(120.0).max(1.0);
        let height = obj.height.unwrap_or(80.0).max(1.0);
        let (_base_w, _base_h, scale) = object_scale_components(obj, width, height);
        scales.push(scale);
    }
    if scales.is_empty() {
        return 1.0;
    }
    scales.iter().sum::<f64>() / scales.len() as f64
}
