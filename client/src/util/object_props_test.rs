use super::*;
use crate::net::types::BoardObject;

fn make_obj(props: serde_json::Value, width: Option<f64>, height: Option<f64>) -> BoardObject {
    BoardObject {
        id: "o1".to_owned(),
        board_id: "b1".to_owned(),
        kind: "rect".to_owned(),
        x: 0.0,
        y: 0.0,
        width,
        height,
        rotation: 0.0,
        z_index: 0,
        version: 1,
        props,
        created_by: None,
    }
}

#[test]
fn value_as_f64_supports_float_and_integer_json_numbers() {
    assert_eq!(value_as_f64(&serde_json::json!(12.5)), Some(12.5));
    assert_eq!(value_as_f64(&serde_json::json!(12)), Some(12.0));
    assert_eq!(value_as_f64(&serde_json::json!("12")), None);
}

#[test]
fn object_scale_components_uses_props_and_clamps_scale() {
    let obj = make_obj(
        serde_json::json!({ "baseWidth": 50, "baseHeight": 25, "scale": 99 }),
        Some(100.0),
        Some(50.0),
    );
    let (base_w, base_h, scale) = object_scale_components(&obj, 100.0, 50.0);
    assert_eq!(base_w, 50.0);
    assert_eq!(base_h, 25.0);
    assert_eq!(scale, 10.0);
}

#[test]
fn object_scale_components_derives_scale_when_missing() {
    let obj = make_obj(
        serde_json::json!({ "baseWidth": 120.0, "baseHeight": 80.0 }),
        Some(120.0),
        Some(80.0),
    );
    let (_base_w, _base_h, scale) = object_scale_components(&obj, 240.0, 80.0);
    assert_eq!(scale, 2.0);
}

#[test]
fn upsert_and_reset_scale_props_initialize_object_maps() {
    let mut obj = make_obj(serde_json::Value::Null, Some(100.0), Some(50.0));
    upsert_object_scale_props(&mut obj, 1.5, 80.0, 40.0);
    assert_eq!(obj.props["scale"], serde_json::json!(1.5));
    assert_eq!(obj.props["baseWidth"], serde_json::json!(80.0));
    assert_eq!(obj.props["baseHeight"], serde_json::json!(40.0));

    let mut props = serde_json::Value::Null;
    reset_scale_props_baseline(&mut props, 0.2, 0.3);
    assert_eq!(props["scale"], serde_json::Value::Null);
    assert_eq!(props["baseWidth"], serde_json::json!(1.0));
    assert_eq!(props["baseHeight"], serde_json::json!(1.0));
}

#[test]
fn reset_wire_object_scale_baseline_uses_wire_defaults() {
    let mut obj = make_obj(serde_json::json!({}), None, None);
    reset_wire_object_scale_baseline(&mut obj);
    assert_eq!(obj.props["baseWidth"], serde_json::json!(120.0));
    assert_eq!(obj.props["baseHeight"], serde_json::json!(80.0));
}

#[test]
fn upsert_color_props_writes_base_shift_fill_and_background() {
    let mut obj = make_obj(serde_json::Value::Null, Some(100.0), Some(80.0));
    upsert_object_color_props(&mut obj, "#ABC", 2.0);
    assert_eq!(obj.props["baseFill"], serde_json::json!("#aabbcc"));
    assert_eq!(obj.props["lightnessShift"], serde_json::json!(1.0));
    assert_eq!(obj.props["fill"], obj.props["backgroundColor"]);
}

#[test]
fn color_accessors_handle_fallbacks() {
    let obj = make_obj(serde_json::json!({ "backgroundColor": "#123456" }), None, None);
    assert_eq!(object_fill_hex(&obj), "#123456");
    assert_eq!(object_base_fill_hex(&obj), "#123456");
    assert_eq!(object_lightness_shift(&obj), 0.0);
}

#[test]
fn apply_lightness_shift_lightens_and_darkens() {
    assert_eq!(apply_lightness_shift_to_hex("#000000", 1.0), "#FFFFFF");
    assert_eq!(apply_lightness_shift_to_hex("#808080", -0.5), "#404040");
    assert_eq!(apply_lightness_shift_to_hex("invalid", 0.0), "#D94B4B");
}

#[test]
fn border_props_roundtrip_and_clamp_width() {
    let mut obj = make_obj(serde_json::Value::Null, None, None);
    upsert_object_border_props(&mut obj, "#abc", 999.0);
    assert_eq!(obj.props["borderColor"], serde_json::json!("#aabbcc"));
    assert_eq!(obj.props["stroke"], serde_json::json!("#aabbcc"));
    assert_eq!(obj.props["borderWidth"], serde_json::json!(24.0));
    assert_eq!(obj.props["stroke_width"], serde_json::json!(24.0));
    assert_eq!(object_border_color_hex(&obj), "#aabbcc");
    assert_eq!(object_border_width(&obj), 24.0);
}

#[test]
fn text_style_props_and_accessors_apply_defaults_and_clamps() {
    let mut obj = make_obj(serde_json::json!({ "fill": "#abcdef" }), None, None);
    upsert_object_text_style_props(&mut obj, "invalid", 3.0);
    assert_eq!(obj.props["textColor"], serde_json::json!("#1f1a17"));
    assert_eq!(obj.props["fontSize"], serde_json::json!(8.0));
    assert_eq!(object_text_color_hex(&obj), "#1f1a17");
    assert_eq!(object_font_size(&obj), 8.0);

    let fallback = make_obj(serde_json::json!({ "fill": "#445566" }), None, None);
    assert_eq!(object_text_color_hex(&fallback), "#445566");
}
