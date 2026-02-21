//! Helpers for reading and updating object property maps.

#[cfg(test)]
#[path = "object_props_test.rs"]
mod object_props_test;

use crate::net::types::BoardObject;
use crate::util::color::{normalize_hex_color, parse_hex_rgb};
use crate::util::dial_math::{BORDER_WIDTH_MAX, BORDER_WIDTH_MIN, TEXT_SIZE_MAX, TEXT_SIZE_MIN};

pub fn value_as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))
}

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

pub fn reset_wire_object_scale_baseline(obj: &mut BoardObject) {
    let width = obj.width.unwrap_or(120.0).max(1.0);
    let height = obj.height.unwrap_or(80.0).max(1.0);
    reset_scale_props_baseline(&mut obj.props, width, height);
}

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
        map.insert("fill".to_owned(), serde_json::Value::String(fill.clone()));
        map.insert("backgroundColor".to_owned(), serde_json::Value::String(fill));
    }
}

pub fn object_fill_hex(obj: &BoardObject) -> String {
    obj.props
        .get("fill")
        .and_then(|v| v.as_str())
        .or_else(|| obj.props.get("backgroundColor").and_then(|v| v.as_str()))
        .or_else(|| obj.props.get("borderColor").and_then(|v| v.as_str()))
        .map(|s| normalize_hex_color(s, "#D94B4B"))
        .unwrap_or_else(|| "#D94B4B".to_owned())
}

pub fn object_base_fill_hex(obj: &BoardObject) -> String {
    obj.props
        .get("baseFill")
        .and_then(|v| v.as_str())
        .map(|s| normalize_hex_color(s, "#D94B4B"))
        .unwrap_or_else(|| object_fill_hex(obj))
}

pub fn object_lightness_shift(obj: &BoardObject) -> f64 {
    obj.props
        .get("lightnessShift")
        .and_then(value_as_f64)
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0)
}

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
        adjusted.round().clamp(0.0, 255.0) as u8
    };
    format!("#{:02x}{:02x}{:02x}", scale(r), scale(g), scale(b))
}

pub fn upsert_object_border_props(obj: &mut BoardObject, border_color: &str, border_width: f64) {
    let color = normalize_hex_color(border_color, "#1F1A17");
    let width = border_width
        .round()
        .clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    if !obj.props.is_object() {
        obj.props = serde_json::json!({});
    }
    if let Some(map) = obj.props.as_object_mut() {
        map.insert("borderColor".to_owned(), serde_json::Value::String(color.clone()));
        map.insert("stroke".to_owned(), serde_json::Value::String(color));
        map.insert("borderWidth".to_owned(), serde_json::json!(width));
        map.insert("stroke_width".to_owned(), serde_json::json!(width));
    }
}

pub fn object_border_color_hex(obj: &BoardObject) -> String {
    obj.props
        .get("borderColor")
        .and_then(|v| v.as_str())
        .or_else(|| obj.props.get("stroke").and_then(|v| v.as_str()))
        .or_else(|| obj.props.get("fill").and_then(|v| v.as_str()))
        .map(|s| normalize_hex_color(s, "#1F1A17"))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

pub fn object_border_width(obj: &BoardObject) -> f64 {
    obj.props
        .get("borderWidth")
        .and_then(value_as_f64)
        .or_else(|| obj.props.get("stroke_width").and_then(value_as_f64))
        .unwrap_or(0.0)
        .clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX)
}

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

pub fn object_text_color_hex(obj: &BoardObject) -> String {
    obj.props
        .get("textColor")
        .and_then(|v| v.as_str())
        .or_else(|| obj.props.get("color").and_then(|v| v.as_str()))
        .or_else(|| obj.props.get("fill").and_then(|v| v.as_str()))
        .map(|s| normalize_hex_color(s, "#1F1A17"))
        .unwrap_or_else(|| "#1F1A17".to_owned())
}

pub fn object_font_size(obj: &BoardObject) -> f64 {
    obj.props
        .get("fontSize")
        .and_then(value_as_f64)
        .unwrap_or(24.0)
        .clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX)
}
