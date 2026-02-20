//! Shared color normalization helpers.

#[cfg(test)]
#[path = "color_test.rs"]
mod color_test;

/// Parse `#RGB` or `#RRGGBB` values into RGB channels.
pub fn parse_hex_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hex = &trimmed[1..];
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

/// Normalize a color to canonical lowercase `#rrggbb`.
pub fn normalize_hex_color(value: &str, fallback: &str) -> String {
    let fallback_rgb = parse_hex_rgb(fallback).unwrap_or((217, 75, 75));
    let (r, g, b) = parse_hex_rgb(value).unwrap_or(fallback_rgb);
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Normalize an optional color value to canonical lowercase `#rrggbb`.
pub fn normalize_hex_color_optional(value: Option<&str>, fallback: &str) -> String {
    value
        .map(|v| normalize_hex_color(v, fallback))
        .unwrap_or_else(|| normalize_hex_color(fallback, fallback))
}
