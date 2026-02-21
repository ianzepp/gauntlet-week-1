#![allow(clippy::clone_on_copy, clippy::float_cmp)]

use uuid::Uuid;

use super::*;
use crate::camera::Point;
use crate::hit::{EdgeEnd, ResizeAnchor};

// =============================================================
// Tool
// =============================================================

#[test]
fn tool_default_is_select() {
    assert_eq!(Tool::default(), Tool::Select);
}

#[test]
fn tool_equality() {
    assert_eq!(Tool::Rect, Tool::Rect);
    assert_ne!(Tool::Rect, Tool::Text);
}

#[test]
fn tool_clone_and_copy() {
    let a = Tool::Diamond;
    let b = a;
    let c = a.clone();
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn tool_debug_format() {
    assert_eq!(format!("{:?}", Tool::Select), "Select");
    assert_eq!(format!("{:?}", Tool::Arrow), "Arrow");
}

#[test]
fn tool_all_variants_distinct() {
    let variants = [
        Tool::Select,
        Tool::Rect,
        Tool::Text,
        Tool::Ellipse,
        Tool::Diamond,
        Tool::Star,
        Tool::Line,
        Tool::Arrow,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn tool_is_shape() {
    assert!(Tool::Rect.is_shape());
    assert!(Tool::Text.is_shape());
    assert!(Tool::Ellipse.is_shape());
    assert!(Tool::Diamond.is_shape());
    assert!(Tool::Star.is_shape());
    assert!(!Tool::Select.is_shape());
    assert!(!Tool::Line.is_shape());
    assert!(!Tool::Arrow.is_shape());
}

#[test]
fn tool_is_edge() {
    assert!(Tool::Line.is_edge());
    assert!(Tool::Arrow.is_edge());
    assert!(!Tool::Select.is_edge());
    assert!(!Tool::Rect.is_edge());
}

// =============================================================
// Modifiers
// =============================================================

#[test]
fn modifiers_default_all_false() {
    let m = Modifiers::default();
    assert!(!m.shift);
    assert!(!m.ctrl);
    assert!(!m.alt);
    assert!(!m.meta);
}

#[test]
fn modifiers_individual_flags() {
    let m = Modifiers { shift: true, ctrl: false, alt: true, meta: false };
    assert!(m.shift);
    assert!(!m.ctrl);
    assert!(m.alt);
    assert!(!m.meta);
}

#[test]
fn modifiers_clone_and_copy() {
    let a = Modifiers { shift: true, ctrl: true, alt: false, meta: false };
    let b = a;
    assert_eq!(b.shift, a.shift);
    assert_eq!(b.ctrl, a.ctrl);
}

// =============================================================
// Button
// =============================================================

#[test]
fn button_equality() {
    assert_eq!(Button::Primary, Button::Primary);
    assert_ne!(Button::Primary, Button::Secondary);
    assert_ne!(Button::Middle, Button::Secondary);
}

#[test]
fn button_all_variants_distinct() {
    let variants = [Button::Primary, Button::Middle, Button::Secondary];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn button_debug_format() {
    assert_eq!(format!("{:?}", Button::Primary), "Primary");
}

// =============================================================
// Key
// =============================================================

#[test]
fn key_equality() {
    assert_eq!(Key("a".into()), Key("a".into()));
    assert_ne!(Key("a".into()), Key("b".into()));
}

#[test]
fn key_clone() {
    let a = Key("Delete".into());
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn key_stores_string() {
    let k = Key("Escape".into());
    assert_eq!(k.0, "Escape");
}

// =============================================================
// WheelDelta
// =============================================================

#[test]
fn wheel_delta_values() {
    let w = WheelDelta { dx: 1.5, dy: -3.0 };
    assert_eq!(w.dx, 1.5);
    assert_eq!(w.dy, -3.0);
}

#[test]
fn wheel_delta_clone_and_copy() {
    let a = WheelDelta { dx: 1.0, dy: 2.0 };
    let b = a;
    assert_eq!(a.dx, b.dx);
    assert_eq!(a.dy, b.dy);
}

// =============================================================
// UiState
// =============================================================

#[test]
fn ui_state_default_tool_is_select() {
    let ui = UiState::default();
    assert_eq!(ui.tool, Tool::Select);
}

#[test]
fn ui_state_default_no_selection() {
    let ui = UiState::default();
    assert!(ui.selected_ids.is_empty());
}

// =============================================================
// InputState
// =============================================================

#[test]
fn input_state_default_is_idle() {
    let s = InputState::default();
    assert!(matches!(s, InputState::Idle));
}

#[test]
fn input_state_panning_carries_point() {
    let s = InputState::Panning { last_screen: Point::new(10.0, 20.0) };
    match s {
        InputState::Panning { last_screen } => {
            assert_eq!(last_screen.x, 10.0);
            assert_eq!(last_screen.y, 20.0);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn input_state_dragging_object_carries_context() {
    let id = Uuid::new_v4();
    let s = InputState::DraggingObject {
        ids: vec![id],
        last_world: Point::new(1.0, 2.0),
        start_world: Point::new(1.0, 2.0),
        originals: vec![(id, 3.0, 4.0)],
        axis_lock: None,
        duplicated: false,
    };
    match s {
        InputState::DraggingObject { ids, originals, .. } => {
            assert_eq!(ids, vec![id]);
            assert_eq!(originals, vec![(id, 3.0, 4.0)]);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn input_state_drawing_shape_carries_anchor() {
    let id = Uuid::new_v4();
    let s = InputState::DrawingShape { id, anchor_world: Point::new(50.0, 60.0) };
    match s {
        InputState::DrawingShape { id: sid, anchor_world } => {
            assert_eq!(sid, id);
            assert_eq!(anchor_world.x, 50.0);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn input_state_resizing_object_carries_context() {
    let id = Uuid::new_v4();
    let s = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: Point::new(0.0, 0.0),
        orig_x: 1.0,
        orig_y: 2.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };
    match s {
        InputState::ResizingObject { anchor, orig_w, orig_h, .. } => {
            assert_eq!(anchor, ResizeAnchor::Se);
            assert_eq!(orig_w, 100.0);
            assert_eq!(orig_h, 80.0);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn input_state_rotating_object_carries_context() {
    let id = Uuid::new_v4();
    let s = InputState::RotatingObject { id, center: Point::new(50.0, 40.0), orig_rotation: 45.0 };
    match s {
        InputState::RotatingObject { center, orig_rotation, .. } => {
            assert_eq!(center.x, 50.0);
            assert_eq!(orig_rotation, 45.0);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn input_state_dragging_edge_endpoint_carries_context() {
    let id = Uuid::new_v4();
    let s = InputState::DraggingEdgeEndpoint { id, end: EdgeEnd::B };
    match s {
        InputState::DraggingEdgeEndpoint { end, .. } => {
            assert_eq!(end, EdgeEnd::B);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn input_state_variants_debug() {
    let id = Uuid::new_v4();
    let variants: Vec<InputState> = vec![
        InputState::Idle,
        InputState::Panning { last_screen: Point::new(0.0, 0.0) },
        InputState::DraggingObject {
            ids: vec![id],
            last_world: Point::new(0.0, 0.0),
            start_world: Point::new(0.0, 0.0),
            originals: vec![(id, 0.0, 0.0)],
            axis_lock: None,
            duplicated: false,
        },
        InputState::DrawingShape { id, anchor_world: Point::new(0.0, 0.0) },
        InputState::ResizingObject {
            id,
            anchor: ResizeAnchor::N,
            start_world: Point::new(0.0, 0.0),
            orig_x: 0.0,
            orig_y: 0.0,
            orig_w: 0.0,
            orig_h: 0.0,
        },
        InputState::RotatingObject { id, center: Point::new(0.0, 0.0), orig_rotation: 0.0 },
        InputState::DraggingEdgeEndpoint { id, end: EdgeEnd::A },
    ];
    for v in &variants {
        let _ = format!("{v:?}");
    }
}
