use super::*;
use crate::state::ui::ToolType;

#[test]
fn placement_shape_returns_expected_shape_defaults() {
    let (kind, w, h, props) = placement_shape(ToolType::Sticky).expect("sticky should have placement");
    assert_eq!(kind, "sticky_note");
    assert_eq!(w, 120.0);
    assert_eq!(h, 120.0);
    assert_eq!(props["title"], serde_json::json!("New note"));

    let (kind, _, _, props) = placement_shape(ToolType::Connector).expect("connector should have placement");
    assert_eq!(kind, "arrow");
    assert_eq!(props["borderColor"], serde_json::json!("#D94B4B"));
}

#[test]
fn placement_shape_returns_none_for_non_shape_tools() {
    assert!(placement_shape(ToolType::Select).is_none());
    assert!(placement_shape(ToolType::Draw).is_none());
    assert!(placement_shape(ToolType::Eraser).is_none());
}

#[test]
fn placement_preview_returns_expected_dims_and_color() {
    let (w, h, color) = placement_preview(ToolType::Frame).expect("frame should have preview");
    assert_eq!(w, 520.0);
    assert_eq!(h, 320.0);
    assert_eq!(color, "rgba(154, 163, 173, 0.20)");
    assert!(placement_preview(ToolType::Select).is_none());
}

#[test]
fn materialize_shape_props_preserves_non_line_values() {
    let props = serde_json::json!({ "keep": true });
    let out = materialize_shape_props("rectangle", 10.0, 20.0, 30.0, 40.0, props.clone());
    assert_eq!(out, props);
}

#[test]
fn materialize_shape_props_adds_line_endpoints_and_preserves_existing_keys() {
    let props = serde_json::json!({ "stroke": "#111111" });
    let out = materialize_shape_props("line", 5.0, 7.0, 9.0, 0.0, props);
    assert_eq!(out["stroke"], serde_json::json!("#111111"));
    assert_eq!(out["a"], serde_json::json!({ "x": 5.0, "y": 7.0 }));
    assert_eq!(out["b"], serde_json::json!({ "x": 14.0, "y": 7.0 }));
}

#[test]
fn materialize_shape_props_coerces_non_object_props_for_arrow() {
    let out = materialize_shape_props("arrow", 1.0, 2.0, 3.0, 0.0, serde_json::Value::Null);
    assert_eq!(out["a"], serde_json::json!({ "x": 1.0, "y": 2.0 }));
    assert_eq!(out["b"], serde_json::json!({ "x": 4.0, "y": 2.0 }));
}
