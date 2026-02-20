//! Selection aggregate helpers for board objects.

#[cfg(test)]
#[path = "selection_metrics_test.rs"]
mod selection_metrics_test;

use crate::state::board::BoardState;
use crate::util::dial_math::{BORDER_WIDTH_MAX, BORDER_WIDTH_MIN, TEXT_SIZE_MAX, TEXT_SIZE_MIN, normalize_degrees_360};
use crate::util::object_props::{
    object_base_fill_hex, object_border_color_hex, object_border_width, object_font_size, object_lightness_shift,
    object_scale_components, object_text_color_hex,
};

pub fn representative_base_color_hex(state: &BoardState) -> String {
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_base_fill_hex))
        .unwrap_or_else(|| "#D94B4B".to_owned())
}

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

pub fn representative_border_color_hex(state: &BoardState) -> String {
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_border_color_hex))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

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

pub fn representative_text_color_hex(state: &BoardState) -> String {
    state
        .selection
        .iter()
        .find_map(|id| state.objects.get(id).map(object_text_color_hex))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

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
