use super::*;

#[test]
fn parse_integer_input_handles_invalid_values() {
    assert_eq!(parse_integer_input("42"), Some(42));
    assert_eq!(parse_integer_input(" 7 "), Some(7));
    assert_eq!(parse_integer_input("1.2"), None);
    assert_eq!(parse_integer_input("abc"), None);
}

#[test]
fn normalize_hex_color_normalizes_valid_inputs() {
    assert_eq!(normalize_hex_color(Some("#ABC".to_owned()), "#000000"), "#aabbcc");
    assert_eq!(normalize_hex_color(Some("#A1B2C3".to_owned()), "#000000"), "#a1b2c3");
}

#[test]
fn normalize_hex_color_falls_back_for_invalid_inputs() {
    assert_eq!(normalize_hex_color(Some("blue".to_owned()), "#ff0000"), "#ff0000");
    assert_eq!(normalize_hex_color(None, "#ff0000"), "#ff0000");
}
