use super::*;

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
    assert_ne!(Tool::Rect, Tool::Ellipse);
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
    assert!(ui.selected_id.is_none());
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
fn input_state_variants_debug() {
    // Just verify all variants exist and Debug works
    let variants: Vec<InputState> = vec![
        InputState::Idle,
        InputState::Panning,
        InputState::DraggingObject,
        InputState::DrawingShape,
        InputState::DraggingEdgeEndpoint,
    ];
    for v in &variants {
        let _ = format!("{v:?}");
    }
}
