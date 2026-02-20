use super::*;

#[test]
fn parse_hex_rgb_supports_short_and_long_forms() {
    assert_eq!(parse_hex_rgb("#ABC"), Some((170, 187, 204)));
    assert_eq!(parse_hex_rgb("  #a1B2c3 "), Some((161, 178, 195)));
}

#[test]
fn parse_hex_rgb_rejects_invalid_inputs() {
    assert_eq!(parse_hex_rgb("AABBCC"), None);
    assert_eq!(parse_hex_rgb("#12"), None);
    assert_eq!(parse_hex_rgb("#abcd"), None);
    assert_eq!(parse_hex_rgb("#12GG34"), None);
}

#[test]
fn normalize_hex_color_uses_canonical_lowercase() {
    assert_eq!(normalize_hex_color("#ABC", "#000000"), "#aabbcc");
    assert_eq!(normalize_hex_color("#A1B2C3", "#000000"), "#a1b2c3");
}

#[test]
fn normalize_hex_color_falls_back_to_input_fallback_or_default() {
    assert_eq!(normalize_hex_color("blue", "#ff0000"), "#ff0000");
    assert_eq!(normalize_hex_color("blue", "invalid"), "#d94b4b");
}

#[test]
fn normalize_hex_color_optional_handles_none() {
    assert_eq!(normalize_hex_color_optional(None, "#ff0000"), "#ff0000");
    assert_eq!(normalize_hex_color_optional(None, "invalid"), "#d94b4b");
}
