//! Helpers for reading and updating object property maps.
//!
//! Object properties are stored as a `serde_json::Value` blob on each `BoardObject`.
//! This module enforces a canonical schema for shared style keys:
//! - `fill`
//! - `stroke`
//! - `strokeWidth`
//! - `textColor`
//! - `fontSize`

#[cfg(test)]
#[path = "object_props_test.rs"]
mod object_props_test;

use crate::net::types::BoardObject;
use crate::util::color::{normalize_hex_color, parse_hex_rgb};
use crate::util::dial_math::{BORDER_WIDTH_MAX, BORDER_WIDTH_MIN, TEXT_SIZE_MAX, TEXT_SIZE_MIN};

/// Coerce a JSON value to `f64`, accepting both floating-point and integer JSON numbers.
///
/// Necessary because `serde_json` serialises integers as `Value::Number` without an `as_f64`
/// shortcut when the original type was `i64`.
#[allow(clippy::cast_precision_loss)]
pub fn value_as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))
}

/// Extract the three scale components needed to resize an object proportionally.
///
/// Returns `(base_width, base_height, scale)` where:
/// - `base_width` / `base_height` are the reference dimensions the object was originally created
///   at (stored as `"baseWidth"` / `"baseHeight"` in props, defaulting to the current dimensions).
/// - `scale` is the current uniform scale factor relative to those base dimensions (stored as
///   `"scale"` in props, or derived from `width / base_width` if absent).
///
/// All three values are clamped to safe ranges (dimensions ≥ 1, scale ∈ [0.1, 10]).
pub fn object_scale_components(obj: &BoardObject, width: f64, height: f64) -> (f64, f64, f64) {
    let base_width = obj
        .props
        .get("baseWidth")
        .and_then(value_as_f64)
        .unwrap_or(width)
        .max(1.0);
    let base_height = obj
        .props
        .get("baseHeight")
        .and_then(value_as_f64)
        .unwrap_or(height)
        .max(1.0);
    let scale = obj
        .props
        .get("scale")
        .and_then(value_as_f64)
        .unwrap_or_else(|| (width / base_width).clamp(0.1, 10.0))
        .clamp(0.1, 10.0);
    (base_width, base_height, scale)
}

/// Write scale state into an object's props map.
///
/// Stores all three values needed to reconstruct the proportional resize state later.
/// Initialises `obj.props` to an empty object if it is not already a JSON object.
pub fn upsert_object_scale_props(obj: &mut BoardObject, scale: f64, base_width: f64, base_height: f64) {
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("scale".to_owned(), serde_json::json!(scale));
        map.insert("baseWidth".to_owned(), serde_json::json!(base_width));
        map.insert("baseHeight".to_owned(), serde_json::json!(base_height));
    }
}

/// Reset the scale baseline of a props map so that the current `width`/`height` become
/// the new reference dimensions and the effective scale returns to 1.0.
///
/// Setting `"scale"` to `null` signals that the scale should be derived from `width/base_width`
/// rather than read from the stored value. Call this after a resize commit to prevent drift.
pub fn reset_scale_props_baseline(props: &mut serde_json::Value, width: f64, height: f64) {
    if !props.is_object() {
        *props = serde_json::json!({});
    }
    if let Some(map) = props.as_object_mut() {
        map.insert("scale".to_owned(), serde_json::Value::Null);
        map.insert("baseWidth".to_owned(), serde_json::json!(width.max(1.0)));
        map.insert("baseHeight".to_owned(), serde_json::json!(height.max(1.0)));
    }
}

/// Reset the scale baseline on a wire `BoardObject`, using its current `width`/`height` fields.
///
/// Convenience wrapper over [`reset_scale_props_baseline`] for objects whose dimensions
/// are stored as `Option<f64>` on the wire type, with safe defaults (120×80).
pub fn reset_wire_object_scale_baseline(obj: &mut BoardObject) {
    let width = obj.width.unwrap_or(120.0).max(1.0);
    let height = obj.height.unwrap_or(80.0).max(1.0);
    reset_scale_props_baseline(&mut obj.props, width, height);
}

/// Write fill color state into an object's props.
///
/// Computes the displayed `fill` color by applying `lightness_shift` to `base_fill`, then
/// stores the value with `"baseFill"` and `"lightnessShift"` for later reconstruction.
pub fn upsert_object_color_props(obj: &mut BoardObject, base_fill: &str, lightness_shift: f64) {
    let base = normalize_hex_color(base_fill, "#D94B4B");
    let shift = lightness_shift.clamp(-1.0, 1.0);
    let fill = apply_lightness_shift_to_hex(&base, shift);
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("baseFill".to_owned(), serde_json::Value::String(base));
        map.insert("lightnessShift".to_owned(), serde_json::json!(shift));
        map.insert("fill".to_owned(), serde_json::Value::String(fill));
    }
}

/// Read the displayed fill color from an object's props.
///
/// Falls back to the application default red (`#D94B4B`) when absent.
pub fn object_fill_hex(obj: &BoardObject) -> String {
    obj.props
        .get("fill")
        .and_then(|v| v.as_str())
        .map_or_else(|| "#D94B4B".to_owned(), |s| normalize_hex_color(s, "#D94B4B"))
}

/// Read the base fill color (before lightness shift) from an object's props.
///
/// Falls back to [`object_fill_hex`] when `"baseFill"` is absent.
pub fn object_base_fill_hex(obj: &BoardObject) -> String {
    obj.props
        .get("baseFill")
        .and_then(|v| v.as_str())
        .map_or_else(|| object_fill_hex(obj), |s| normalize_hex_color(s, "#D94B4B"))
}

/// Read the lightness shift value from an object's props, clamped to [-1, 1].
///
/// Returns 0.0 (no shift) when the prop is absent.
pub fn object_lightness_shift(obj: &BoardObject) -> f64 {
    obj.props
        .get("lightnessShift")
        .and_then(value_as_f64)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0)
}

/// Shift the lightness of a hex color by a signed factor in [-1, 1].
///
/// Positive `shift` moves each RGB channel toward 255 (lighter).
/// Negative `shift` scales each channel toward 0 (darker).
/// The formula is linear: lighter uses `channel + (255 - channel) * shift`,
/// darker uses `channel * (1 + shift)`, which keeps black at black.
/// Returns a lowercase `#rrggbb` string. Defaults to `#D94B4B` on parse failure.
pub fn apply_lightness_shift_to_hex(base_hex: &str, shift: f64) -> String {
    let (r, g, b) = parse_hex_rgb(base_hex).unwrap_or((217, 75, 75));
    let shift = shift.clamp(-1.0, 1.0);
    let scale = |channel: u8| -> u8 {
        let current = f64::from(channel);
        let adjusted = if shift >= 0.0 {
            current + ((255.0 - current) * shift)
        } else {
            current * (1.0 + shift)
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            adjusted.round().clamp(0.0, 255.0) as u8
        }
    };
    format!("#{:02x}{:02x}{:02x}", scale(r), scale(g), scale(b))
}

/// Write border color and width into an object's props.
///
/// Stores canonical keys `"stroke"` and `"strokeWidth"`. Width is rounded to the nearest pixel
/// and clamped to [`BORDER_WIDTH_MIN`]..=[`BORDER_WIDTH_MAX`].
pub fn upsert_object_border_props(obj: &mut BoardObject, border_color: &str, border_width: f64) {
    let color = normalize_hex_color(border_color, "#1F1A17");
    let width = border_width
        .round()
        .clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("stroke".to_owned(), serde_json::Value::String(color));
        map.insert("strokeWidth".to_owned(), serde_json::json!(width));
    }
}

/// Read the border color from an object's props.
///
/// Defaults to dark charcoal (`#1F1A17`) when absent.
pub fn object_border_color_hex(obj: &BoardObject) -> String {
    obj.props
        .get("stroke")
        .and_then(|v| v.as_str())
        .map_or_else(|| "#1F1A17".to_owned(), |s| normalize_hex_color(s, "#1F1A17"))
}

/// Read the border width from an object's props, clamped to [`BORDER_WIDTH_MIN`]..=[`BORDER_WIDTH_MAX`].
///
/// Returns 0 when absent.
pub fn object_border_width(obj: &BoardObject) -> f64 {
    obj.props
        .get("strokeWidth")
        .and_then(value_as_f64)
        .unwrap_or(0.0)
        .clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

/// Write text color and font size into an object's props.
///
/// Uses the canonical keys `"textColor"` and `"fontSize"`. Font size is rounded to the nearest
/// pixel and clamped to [`TEXT_SIZE_MIN`]..=[`TEXT_SIZE_MAX`].
pub fn upsert_object_text_style_props(obj: &mut BoardObject, text_color: &str, font_size: f64) {
    let color = normalize_hex_color(text_color, "#1F1A17");
    let size = font_size.round().clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX);
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("textColor".to_owned(), serde_json::Value::String(color));
        map.insert("fontSize".to_owned(), serde_json::json!(size));
    }
}

/// Read the text color from an object's props.
///
/// Defaults to dark charcoal (`#1F1A17`) when absent.
pub fn object_text_color_hex(obj: &BoardObject) -> String {
    obj.props
        .get("textColor")
        .and_then(|v| v.as_str())
        .map_or_else(|| "#1F1A17".to_owned(), |s| normalize_hex_color(s, "#1F1A17"))
}

/// Read the font size from an object's props, clamped to [`TEXT_SIZE_MIN`]..=[`TEXT_SIZE_MAX`].
///
/// Returns 24.0 (the default body text size) when the prop is absent.
pub fn object_font_size(obj: &BoardObject) -> f64 {
    obj.props
        .get("fontSize")
        .and_then(value_as_f64)
        .unwrap_or(24.0)
        .clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX)
}
