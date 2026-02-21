use std::collections::HashSet;

use super::*;
use crate::net::types::BoardObject;
use crate::state::board::BoardState;

fn make_obj(id: &str, rotation: f64, width: Option<f64>, height: Option<f64>, props: serde_json::Value) -> BoardObject {
    BoardObject {
        id: id.to_owned(),
        board_id: "b1".to_owned(),
        kind: "rect".to_owned(),
        x: 0.0,
        y: 0.0,
        width,
        height,
        rotation,
        z_index: 0,
        version: 1,
        group_id: None,
        props,
        created_by: None,
    }
}

fn selection(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

#[test]
fn representative_color_and_text_accessors_use_selected_objects() {
    let mut state = BoardState::default();
    state.objects.insert(
        "a".to_owned(),
        make_obj(
            "a",
            0.0,
            Some(100.0),
            Some(100.0),
            serde_json::json!({
                "baseFill": "#112233",
                "borderColor": "#223344",
                "textColor": "#334455"
            }),
        ),
    );
    state.selection = selection(&["a"]);

    assert_eq!(representative_base_color_hex(&state), "#112233");
    assert_eq!(representative_border_color_hex(&state), "#223344");
    assert_eq!(representative_text_color_hex(&state), "#334455");
}

#[test]
fn representative_lightness_and_border_width_average_and_clamp() {
    let mut state = BoardState::default();
    state.objects.insert(
        "a".to_owned(),
        make_obj(
            "a",
            0.0,
            None,
            None,
            serde_json::json!({ "lightnessShift": -2, "borderWidth": 2 }),
        ),
    );
    state.objects.insert(
        "b".to_owned(),
        make_obj(
            "b",
            0.0,
            None,
            None,
            serde_json::json!({ "lightnessShift": 1, "borderWidth": 10 }),
        ),
    );
    state.selection = selection(&["a", "b"]);

    assert_eq!(representative_lightness_shift(&state), -0.0);
    assert_eq!(representative_border_width(&state), 6.0);
}

#[test]
fn representative_font_size_rounds_mean() {
    let mut state = BoardState::default();
    state.objects.insert(
        "a".to_owned(),
        make_obj("a", 0.0, None, None, serde_json::json!({ "fontSize": 17 })),
    );
    state.objects.insert(
        "b".to_owned(),
        make_obj("b", 0.0, None, None, serde_json::json!({ "fontSize": 18 })),
    );
    state.selection = selection(&["a", "b"]);

    assert_eq!(representative_font_size(&state), 18.0);
}

#[test]
fn representative_rotation_handles_wraparound_mean() {
    let mut state = BoardState::default();
    state
        .objects
        .insert("a".to_owned(), make_obj("a", 350.0, None, None, serde_json::json!({})));
    state
        .objects
        .insert("b".to_owned(), make_obj("b", 10.0, None, None, serde_json::json!({})));
    state.selection = selection(&["a", "b"]);

    let rotation = representative_rotation_deg(&state);
    assert!(rotation < 1.0 || rotation > 359.0);
}

#[test]
fn representative_scale_factor_averages_scales_from_props_and_geometry() {
    let mut state = BoardState::default();
    state.objects.insert(
        "a".to_owned(),
        make_obj(
            "a",
            0.0,
            Some(200.0),
            Some(100.0),
            serde_json::json!({ "baseWidth": 100.0, "baseHeight": 50.0 }),
        ),
    );
    state.objects.insert(
        "b".to_owned(),
        make_obj("b", 0.0, Some(100.0), Some(80.0), serde_json::json!({ "scale": 3.0 })),
    );
    state.selection = selection(&["a", "b"]);

    assert_eq!(representative_scale_factor(&state), 2.5);
}

#[test]
fn representative_functions_return_defaults_without_selected_objects() {
    let mut state = BoardState::default();
    state.selection = selection(&["missing"]);

    assert_eq!(representative_base_color_hex(&state), "#D94B4B");
    assert_eq!(representative_lightness_shift(&state), 0.0);
    assert_eq!(representative_border_color_hex(&state), "#1F1A17");
    assert_eq!(representative_border_width(&state), 1.0);
    assert_eq!(representative_text_color_hex(&state), "#1F1A17");
    assert_eq!(representative_font_size(&state), 24.0);
    assert_eq!(representative_rotation_deg(&state), 0.0);
    assert_eq!(representative_scale_factor(&state), 1.0);
}
