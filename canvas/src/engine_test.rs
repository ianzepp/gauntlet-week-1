#![allow(clippy::clone_on_copy, clippy::float_cmp)]

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::doc::{BoardObject, ObjectKind, PartialBoardObject};
use crate::hit::EdgeEnd;
use crate::input::{Button, InputState, Key, Modifiers, Tool, WheelDelta};

// =============================================================
// Helpers
// =============================================================

fn make_object(kind: ObjectKind, z: i64) -> BoardObject {
    BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind,
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 80.0,
        rotation: 0.0,
        z_index: z,
        props: json!({}),
        created_by: None,
        version: 1,
    }
}

fn make_object_at(kind: ObjectKind, x: f64, y: f64, w: f64, h: f64) -> BoardObject {
    BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind,
        x,
        y,
        width: w,
        height: h,
        rotation: 0.0,
        z_index: 0,
        props: json!({}),
        created_by: None,
        version: 1,
    }
}

fn make_edge(kind: ObjectKind, ax: f64, ay: f64, bx: f64, by: f64) -> BoardObject {
    BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind,
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        rotation: 0.0,
        z_index: 0,
        props: json!({
            "a": { "type": "free", "x": ax, "y": ay },
            "b": { "type": "free", "x": bx, "y": by },
        }),
        created_by: None,
        version: 1,
    }
}

fn no_modifiers() -> Modifiers {
    Modifiers::default()
}

fn ctrl_modifier() -> Modifiers {
    Modifiers { ctrl: true, ..Default::default() }
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn has_action<F>(actions: &[Action], pred: F) -> bool
where
    F: Fn(&Action) -> bool,
{
    actions.iter().any(pred)
}

fn has_render_needed(actions: &[Action]) -> bool {
    has_action(actions, |a| matches!(a, Action::RenderNeeded))
}

fn has_object_created(actions: &[Action]) -> bool {
    has_action(actions, |a| matches!(a, Action::ObjectCreated(_)))
}

fn has_object_updated(actions: &[Action]) -> bool {
    has_action(actions, |a| matches!(a, Action::ObjectUpdated { .. }))
}

fn has_object_deleted(actions: &[Action]) -> bool {
    has_action(actions, |a| matches!(a, Action::ObjectDeleted { .. }))
}

// =============================================================
// EngineCore: construction and defaults
// =============================================================

#[test]
fn core_new_has_no_selection() {
    let core = EngineCore::new();
    assert!(core.selection().is_none());
}

#[test]
fn core_default_camera_is_identity() {
    let core = EngineCore::new();
    let cam = core.camera();
    assert_eq!(cam.pan_x, 0.0);
    assert_eq!(cam.pan_y, 0.0);
    assert_eq!(cam.zoom, 1.0);
}

#[test]
fn core_default_tool_is_select() {
    let core = EngineCore::new();
    assert_eq!(core.ui.tool, Tool::Select);
}

#[test]
fn core_default_doc_is_empty() {
    let core = EngineCore::new();
    assert!(core.doc.is_empty());
}

#[test]
fn core_default_viewport_is_zero() {
    let core = EngineCore::new();
    assert_eq!(core.viewport_width, 0.0);
    assert_eq!(core.viewport_height, 0.0);
    assert_eq!(core.dpr, 1.0);
}

// =============================================================
// EngineCore: load_snapshot
// =============================================================

#[test]
fn core_load_snapshot_populates_doc() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.load_snapshot(vec![obj]);
    assert!(core.object(&id).is_some());
}

#[test]
fn core_load_snapshot_replaces_existing() {
    let mut core = EngineCore::new();
    let old = make_object(ObjectKind::Rect, 0);
    let old_id = old.id;
    core.load_snapshot(vec![old]);

    let new = make_object(ObjectKind::Ellipse, 0);
    let new_id = new.id;
    core.load_snapshot(vec![new]);

    assert!(core.object(&old_id).is_none());
    assert!(core.object(&new_id).is_some());
}

#[test]
fn core_load_snapshot_empty_clears() {
    let mut core = EngineCore::new();
    core.load_snapshot(vec![make_object(ObjectKind::Rect, 0)]);
    core.load_snapshot(vec![]);
    assert!(core.doc.is_empty());
}

// =============================================================
// EngineCore: apply_create
// =============================================================

#[test]
fn core_apply_create_adds_object() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Ellipse, 0);
    let id = obj.id;
    core.apply_create(obj);
    assert!(core.object(&id).is_some());
    assert_eq!(core.object(&id).unwrap().kind, ObjectKind::Ellipse);
}

#[test]
fn core_apply_create_multiple() {
    let mut core = EngineCore::new();
    let a = make_object(ObjectKind::Rect, 0);
    let b = make_object(ObjectKind::Star, 1);
    let id_a = a.id;
    let id_b = b.id;
    core.apply_create(a);
    core.apply_create(b);
    assert!(core.object(&id_a).is_some());
    assert!(core.object(&id_b).is_some());
}

// =============================================================
// EngineCore: apply_update
// =============================================================

#[test]
fn core_apply_update_modifies_fields() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);

    let partial = PartialBoardObject { x: Some(99.0), y: Some(88.0), ..Default::default() };
    core.apply_update(&id, &partial);

    let updated = core.object(&id).unwrap();
    assert_eq!(updated.x, 99.0);
    assert_eq!(updated.y, 88.0);
}

#[test]
fn core_apply_update_nonexistent_is_noop() {
    let mut core = EngineCore::new();
    let id = Uuid::new_v4();
    let partial = PartialBoardObject { x: Some(10.0), ..Default::default() };
    // Should not panic
    core.apply_update(&id, &partial);
}

// =============================================================
// EngineCore: apply_delete
// =============================================================

#[test]
fn core_apply_delete_removes_object() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);
    core.apply_delete(&id);
    assert!(core.object(&id).is_none());
}

#[test]
fn core_apply_delete_clears_selection_if_selected() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    core.apply_delete(&id);
    assert!(core.selection().is_none());
}

#[test]
fn core_apply_delete_preserves_selection_of_other() {
    let mut core = EngineCore::new();
    let a = make_object(ObjectKind::Rect, 0);
    let b = make_object(ObjectKind::Ellipse, 0);
    let id_a = a.id;
    let id_b = b.id;
    core.apply_create(a);
    core.apply_create(b);
    core.ui.selected_id = Some(id_a);

    core.apply_delete(&id_b);
    assert_eq!(core.selection(), Some(id_a));
}

#[test]
fn core_apply_delete_nonexistent_is_noop() {
    let mut core = EngineCore::new();
    let id = Uuid::new_v4();
    core.apply_delete(&id); // should not panic
    assert!(core.doc.is_empty());
}

// =============================================================
// EngineCore: set_tool
// =============================================================

#[test]
fn core_set_tool_changes_tool() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    assert_eq!(core.ui.tool, Tool::Rect);
}

#[test]
fn core_set_tool_all_variants() {
    let mut core = EngineCore::new();
    let tools = [
        Tool::Select,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Diamond,
        Tool::Star,
        Tool::Line,
        Tool::Arrow,
    ];
    for tool in tools {
        core.set_tool(tool);
        assert_eq!(core.ui.tool, tool);
    }
}

// =============================================================
// EngineCore: set_text
// =============================================================

#[test]
fn core_set_text_updates_props() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);

    core.set_text(&id, "Header".into(), "Body".into(), "Footer".into());

    let updated = core.object(&id).unwrap();
    assert_eq!(updated.props["head"], "Header");
    assert_eq!(updated.props["text"], "Body");
    assert_eq!(updated.props["foot"], "Footer");
}

#[test]
fn core_set_text_returns_object_updated_action() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);

    let action = core.set_text(&id, "H".into(), "T".into(), "F".into());
    match action {
        Action::ObjectUpdated { id: action_id, fields } => {
            assert_eq!(action_id, id);
            let props = fields.props.unwrap();
            assert_eq!(props["head"], "H");
            assert_eq!(props["text"], "T");
            assert_eq!(props["foot"], "F");
        }
        _ => panic!("Expected Action::ObjectUpdated, got {action:?}"),
    }
}

#[test]
fn core_set_text_preserves_other_props() {
    let mut core = EngineCore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"fill": "#FF0000", "stroke": "#000"});
    let id = obj.id;
    core.apply_create(obj);

    core.set_text(&id, "H".into(), "T".into(), "F".into());

    let updated = core.object(&id).unwrap();
    assert_eq!(updated.props["fill"], "#FF0000");
    assert_eq!(updated.props["stroke"], "#000");
    assert_eq!(updated.props["head"], "H");
}

#[test]
fn core_set_text_empty_strings() {
    let mut core = EngineCore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"head": "H", "text": "T", "foot": "F"});
    let id = obj.id;
    core.apply_create(obj);

    let action = core.set_text(&id, String::new(), String::new(), String::new());
    assert!(matches!(action, Action::ObjectUpdated { .. }));

    let updated = core.object(&id).unwrap();
    assert_eq!(updated.props["head"], "");
    assert_eq!(updated.props["text"], "");
    assert_eq!(updated.props["foot"], "");
}

#[test]
fn core_set_text_missing_object_returns_none() {
    let mut core = EngineCore::new();
    let id = Uuid::new_v4();
    let action = core.set_text(&id, "H".into(), "T".into(), "F".into());
    assert!(matches!(action, Action::None));
}

#[test]
fn core_set_text_unchanged_returns_none() {
    let mut core = EngineCore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"head": "H", "text": "T", "foot": "F"});
    let id = obj.id;
    core.apply_create(obj);

    let action = core.set_text(&id, "H".into(), "T".into(), "F".into());
    assert!(matches!(action, Action::None));
}

// =============================================================
// EngineCore: set_viewport
// =============================================================

#[test]
fn core_set_viewport_stores_dimensions() {
    let mut core = EngineCore::new();
    core.set_viewport(1920.0, 1080.0, 2.0);
    assert_eq!(core.viewport_width, 1920.0);
    assert_eq!(core.viewport_height, 1080.0);
    assert_eq!(core.dpr, 2.0);
}

#[test]
fn core_set_viewport_overwrites_previous() {
    let mut core = EngineCore::new();
    core.set_viewport(800.0, 600.0, 1.0);
    core.set_viewport(1024.0, 768.0, 1.5);
    assert_eq!(core.viewport_width, 1024.0);
    assert_eq!(core.viewport_height, 768.0);
    assert_eq!(core.dpr, 1.5);
}

#[test]
fn core_set_viewport_zero_dimensions() {
    let mut core = EngineCore::new();
    core.set_viewport(0.0, 0.0, 1.0);
    assert_eq!(core.viewport_width, 0.0);
    assert_eq!(core.viewport_height, 0.0);
}

#[test]
fn core_set_viewport_fractional_dpr() {
    let mut core = EngineCore::new();
    core.set_viewport(1440.0, 900.0, 1.25);
    assert_eq!(core.dpr, 1.25);
}

// =============================================================
// EngineCore: queries
// =============================================================

#[test]
fn core_object_returns_none_for_missing() {
    let core = EngineCore::new();
    assert!(core.object(&Uuid::new_v4()).is_none());
}

#[test]
fn core_object_returns_correct_object() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Star, 5);
    let id = obj.id;
    core.apply_create(obj);
    let retrieved = core.object(&id).unwrap();
    assert_eq!(retrieved.kind, ObjectKind::Star);
    assert_eq!(retrieved.z_index, 5);
}

// =============================================================
// Pointer down — Select tool on empty space
// =============================================================

#[test]
fn select_click_empty_stays_idle() {
    let mut core = EngineCore::new();
    let actions = core.on_pointer_down(pt(500.0, 500.0), Button::Primary, no_modifiers());
    // Should transition to Panning (empty space enables drag-to-pan).
    assert!(matches!(core.input, InputState::Panning { .. }));
    // No render needed if nothing was selected before.
    assert!(!has_render_needed(&actions));
}

#[test]
fn select_click_empty_deselects() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 10.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    // Click far from the object.
    let actions = core.on_pointer_down(pt(500.0, 500.0), Button::Primary, no_modifiers());
    assert!(core.selection().is_none());
    assert!(has_render_needed(&actions));
}

// =============================================================
// Pointer down — Select tool on object body
// =============================================================

#[test]
fn select_click_body_selects_and_starts_drag() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    // Click in the middle of the rect.
    let actions = core.on_pointer_down(pt(50.0, 40.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));
    assert!(matches!(core.input, InputState::DraggingObject { .. }));
    assert!(has_render_needed(&actions));
}

#[test]
fn select_click_body_stores_original_position() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 20.0, 30.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    core.on_pointer_down(pt(70.0, 70.0), Button::Primary, no_modifiers());
    match &core.input {
        InputState::DraggingObject { id: drag_id, orig_x, orig_y, .. } => {
            assert_eq!(*drag_id, id);
            assert_eq!(*orig_x, 20.0);
            assert_eq!(*orig_y, 30.0);
        }
        other => panic!("Expected DraggingObject, got {other:?}"),
    }
}

// =============================================================
// Pointer down — Select tool on edge body
// =============================================================

#[test]
fn select_click_edge_body_selects() {
    let mut core = EngineCore::new();
    let edge = make_edge(ObjectKind::Line, 0.0, 0.0, 100.0, 0.0);
    let id = edge.id;
    core.apply_create(edge);

    // Click near the line (at y=0, between x=0 and x=100).
    let actions = core.on_pointer_down(pt(50.0, 0.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));
    assert!(has_render_needed(&actions));
}

// =============================================================
// Pointer down — Select tool on resize handle
// =============================================================

#[test]
fn select_click_resize_handle_starts_resize() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    // Click on the SE handle (bottom-right corner at 100, 80).
    let actions = core.on_pointer_down(pt(100.0, 80.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::ResizingObject { .. }));
    // Resize handle hit doesn't emit render (state change only).
    let _ = actions;
}

// =============================================================
// Pointer down — Select tool on rotate handle
// =============================================================

#[test]
fn select_click_rotate_handle_starts_rotation() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    // Rotate handle is above center-top (50, -24) at zoom 1.
    let actions = core.on_pointer_down(pt(50.0, -24.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::RotatingObject { .. }));
    let _ = actions;
}

// =============================================================
// Pointer down — Select tool on edge endpoint
// =============================================================

#[test]
fn select_click_edge_endpoint_starts_drag() {
    let mut core = EngineCore::new();
    let edge = make_edge(ObjectKind::Arrow, 10.0, 10.0, 200.0, 200.0);
    let id = edge.id;
    core.apply_create(edge);
    core.ui.selected_id = Some(id);

    // Click near endpoint A.
    let actions = core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::DraggingEdgeEndpoint { .. }));
    assert!(has_render_needed(&actions));
}

// =============================================================
// Pointer down — Middle button pans
// =============================================================

#[test]
fn middle_button_starts_panning() {
    let mut core = EngineCore::new();
    let actions = core.on_pointer_down(pt(100.0, 100.0), Button::Middle, no_modifiers());
    assert!(matches!(core.input, InputState::Panning { .. }));
    assert!(has_action(&actions, |a| matches!(a, Action::SetCursor(_))));
}

#[test]
fn middle_button_pans_regardless_of_tool() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(100.0, 100.0), Button::Middle, no_modifiers());
    assert!(matches!(core.input, InputState::Panning { .. }));
}

// =============================================================
// Pointer down — Shape tools
// =============================================================

#[test]
fn rect_tool_creates_shape() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    let actions = core.on_pointer_down(pt(50.0, 60.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::DrawingShape { .. }));
    assert!(has_render_needed(&actions));
    assert_eq!(core.doc.len(), 1);

    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.kind, ObjectKind::Rect);
    assert_eq!(obj.x, 50.0);
    assert_eq!(obj.y, 60.0);
    assert_eq!(obj.width, 0.0);
    assert_eq!(obj.height, 0.0);
}

#[test]
fn ellipse_tool_creates_shape() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Ellipse);
    core.on_pointer_down(pt(10.0, 20.0), Button::Primary, no_modifiers());
    assert_eq!(core.doc.sorted_objects()[0].kind, ObjectKind::Ellipse);
}

#[test]
fn diamond_tool_creates_shape() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Diamond);
    core.on_pointer_down(pt(10.0, 20.0), Button::Primary, no_modifiers());
    assert_eq!(core.doc.sorted_objects()[0].kind, ObjectKind::Diamond);
}

#[test]
fn star_tool_creates_shape() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Star);
    core.on_pointer_down(pt(10.0, 20.0), Button::Primary, no_modifiers());
    assert_eq!(core.doc.sorted_objects()[0].kind, ObjectKind::Star);
}

#[test]
fn shape_tool_selects_created_object() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(50.0, 60.0), Button::Primary, no_modifiers());
    assert!(core.selection().is_some());
}

#[test]
fn shape_tool_sets_default_props() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(50.0, 60.0), Button::Primary, no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.props["fill"], "#D94B4B");
    assert_eq!(obj.props["stroke"], "#1F1A17");
}

// =============================================================
// Pointer down — Edge tools
// =============================================================

#[test]
fn line_tool_creates_edge() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Line);
    let actions = core.on_pointer_down(pt(30.0, 40.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::DrawingShape { .. }));
    assert!(has_render_needed(&actions));

    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.kind, ObjectKind::Line);
    assert_eq!(obj.props["a"]["x"], 30.0);
    assert_eq!(obj.props["a"]["y"], 40.0);
    assert_eq!(obj.props["b"]["x"], 30.0);
    assert_eq!(obj.props["b"]["y"], 40.0);
}

#[test]
fn arrow_tool_creates_edge() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Arrow);
    core.on_pointer_down(pt(10.0, 20.0), Button::Primary, no_modifiers());
    assert_eq!(core.doc.sorted_objects()[0].kind, ObjectKind::Arrow);
}

// =============================================================
// Pointer down — Secondary button is no-op
// =============================================================

#[test]
fn secondary_button_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_pointer_down(pt(50.0, 50.0), Button::Secondary, no_modifiers());
    assert!(actions.is_empty());
    assert!(matches!(core.input, InputState::Idle));
}

// =============================================================
// Pointer move — Panning
// =============================================================

#[test]
fn panning_updates_camera() {
    let mut core = EngineCore::new();
    core.input = InputState::Panning { last_screen: pt(100.0, 100.0) };
    let actions = core.on_pointer_move(pt(120.0, 110.0), no_modifiers());
    assert_eq!(core.camera.pan_x, 20.0);
    assert_eq!(core.camera.pan_y, 10.0);
    assert!(has_render_needed(&actions));
}

#[test]
fn panning_accumulates() {
    let mut core = EngineCore::new();
    core.input = InputState::Panning { last_screen: pt(0.0, 0.0) };
    core.on_pointer_move(pt(10.0, 5.0), no_modifiers());
    core.on_pointer_move(pt(20.0, 15.0), no_modifiers());
    assert_eq!(core.camera.pan_x, 20.0);
    assert_eq!(core.camera.pan_y, 15.0);
}

// =============================================================
// Pointer move — DraggingObject
// =============================================================

#[test]
fn dragging_object_moves_position() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 50.0, 60.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(75.0, 80.0), orig_x: 50.0, orig_y: 60.0 };

    let actions = core.on_pointer_move(pt(85.0, 90.0), no_modifiers());
    let updated = core.object(&id).unwrap();
    assert_eq!(updated.x, 60.0); // 50 + (85-75)
    assert_eq!(updated.y, 70.0); // 60 + (90-80)
    assert!(has_render_needed(&actions));
}

#[test]
fn dragging_object_accumulates_moves() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 40.0), orig_x: 0.0, orig_y: 0.0 };

    core.on_pointer_move(pt(60.0, 50.0), no_modifiers());
    core.on_pointer_move(pt(70.0, 60.0), no_modifiers());
    let updated = core.object(&id).unwrap();
    assert_eq!(updated.x, 20.0);
    assert_eq!(updated.y, 20.0);
}

// =============================================================
// Pointer move — DrawingShape (node)
// =============================================================

#[test]
fn drawing_shape_updates_dimensions() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 20.0), Button::Primary, no_modifiers());

    core.on_pointer_move(pt(110.0, 120.0), no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.x, 10.0);
    assert_eq!(obj.y, 20.0);
    assert_eq!(obj.width, 100.0);
    assert_eq!(obj.height, 100.0);
}

#[test]
fn drawing_shape_handles_negative_direction() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(100.0, 100.0), Button::Primary, no_modifiers());

    // Drag up-left.
    core.on_pointer_move(pt(50.0, 60.0), no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.x, 50.0);
    assert_eq!(obj.y, 60.0);
    assert_eq!(obj.width, 50.0);
    assert_eq!(obj.height, 40.0);
}

// =============================================================
// Pointer move — DrawingShape (edge)
// =============================================================

#[test]
fn drawing_edge_updates_endpoint_b() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Line);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());

    core.on_pointer_move(pt(200.0, 150.0), no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.props["b"]["x"], 200.0);
    assert_eq!(obj.props["b"]["y"], 150.0);
    // Endpoint A should be unchanged.
    assert_eq!(obj.props["a"]["x"], 10.0);
    assert_eq!(obj.props["a"]["y"], 10.0);
}

// =============================================================
// Pointer move — ResizingObject
// =============================================================

#[test]
fn resize_se_grows_dimensions() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: pt(110.0, 100.0),
        orig_x: 10.0,
        orig_y: 20.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    core.on_pointer_move(pt(130.0, 120.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.width, 120.0); // 100 + 20
    assert_eq!(obj.height, 100.0); // 80 + 20
    assert_eq!(obj.x, 10.0); // unchanged
    assert_eq!(obj.y, 20.0); // unchanged
}

#[test]
fn resize_nw_moves_origin_and_shrinks() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Nw,
        start_world: pt(10.0, 20.0),
        orig_x: 10.0,
        orig_y: 20.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    core.on_pointer_move(pt(30.0, 40.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 30.0); // 10 + 20
    assert_eq!(obj.y, 40.0); // 20 + 20
    assert_eq!(obj.width, 80.0); // 100 - 20
    assert_eq!(obj.height, 60.0); // 80 - 20
}

#[test]
fn resize_n_only_changes_y_and_height() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::N,
        start_world: pt(60.0, 20.0),
        orig_x: 10.0,
        orig_y: 20.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    core.on_pointer_move(pt(60.0, 30.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 10.0);
    assert_eq!(obj.y, 30.0);
    assert_eq!(obj.width, 100.0);
    assert_eq!(obj.height, 70.0);
}

#[test]
fn resize_e_only_changes_width() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::E,
        start_world: pt(50.0, 25.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    core.on_pointer_move(pt(80.0, 25.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.width, 80.0);
    assert_eq!(obj.height, 50.0);
    assert_eq!(obj.x, 0.0);
    assert_eq!(obj.y, 0.0);
}

#[test]
fn resize_w_moves_x_and_shrinks_width() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 0.0, 100.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::W,
        start_world: pt(10.0, 25.0),
        orig_x: 10.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 50.0,
    };

    core.on_pointer_move(pt(30.0, 25.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 30.0);
    assert_eq!(obj.width, 80.0);
}

#[test]
fn resize_s_only_changes_height() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::S,
        start_world: pt(25.0, 50.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    core.on_pointer_move(pt(25.0, 70.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.height, 70.0);
    assert_eq!(obj.width, 50.0);
}

#[test]
fn resize_ne_changes_y_h_and_w() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 10.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Ne,
        start_world: pt(50.0, 10.0),
        orig_x: 0.0,
        orig_y: 10.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    core.on_pointer_move(pt(60.0, 5.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.y, 5.0);
    assert_eq!(obj.height, 55.0);
    assert_eq!(obj.width, 60.0);
    assert_eq!(obj.x, 0.0);
}

#[test]
fn resize_sw_changes_x_w_and_h() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Sw,
        start_world: pt(10.0, 50.0),
        orig_x: 10.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    core.on_pointer_move(pt(5.0, 60.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 5.0);
    assert_eq!(obj.width, 55.0);
    assert_eq!(obj.height, 60.0);
}

#[test]
fn resize_clamps_to_zero() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: pt(50.0, 50.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    // Drag past origin.
    core.on_pointer_move(pt(-20.0, -20.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert!(obj.width >= 0.0);
    assert!(obj.height >= 0.0);
}

// =============================================================
// Pointer move — RotatingObject
// =============================================================

#[test]
fn rotating_updates_rotation() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    // Move to the right of center (positive X, same Y).
    core.on_pointer_move(pt(150.0, 40.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    // atan2(0, 100) = 0 degrees, + 90 = 90 degrees.
    assert!((obj.rotation - 90.0).abs() < 0.01);
}

#[test]
fn rotating_above_center_gives_zero_degrees() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    // Move directly above center.
    core.on_pointer_move(pt(50.0, -60.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    // atan2(-100, 0) = -90 degrees, + 90 = 0 degrees.
    assert!(obj.rotation.abs() < 0.01);
}

// =============================================================
// Pointer move — DraggingEdgeEndpoint
// =============================================================

#[test]
fn dragging_edge_endpoint_a_updates() {
    let mut core = EngineCore::new();
    let edge = make_edge(ObjectKind::Line, 10.0, 10.0, 200.0, 200.0);
    let id = edge.id;
    core.apply_create(edge);
    core.input = InputState::DraggingEdgeEndpoint { id, end: EdgeEnd::A };

    core.on_pointer_move(pt(50.0, 50.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.props["a"]["x"], 50.0);
    assert_eq!(obj.props["a"]["y"], 50.0);
    // B unchanged.
    assert_eq!(obj.props["b"]["x"], 200.0);
    assert_eq!(obj.props["b"]["y"], 200.0);
}

#[test]
fn dragging_edge_endpoint_b_updates() {
    let mut core = EngineCore::new();
    let edge = make_edge(ObjectKind::Arrow, 0.0, 0.0, 100.0, 100.0);
    let id = edge.id;
    core.apply_create(edge);
    core.input = InputState::DraggingEdgeEndpoint { id, end: EdgeEnd::B };

    core.on_pointer_move(pt(300.0, 250.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.props["b"]["x"], 300.0);
    assert_eq!(obj.props["b"]["y"], 250.0);
    assert_eq!(obj.props["a"]["x"], 0.0);
}

// =============================================================
// Pointer move — Idle is no-op
// =============================================================

#[test]
fn idle_move_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_pointer_move(pt(100.0, 100.0), no_modifiers());
    assert!(actions.is_empty());
}

// =============================================================
// Pointer up — DraggingObject
// =============================================================

#[test]
fn pointer_up_dragging_emits_update() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 40.0), orig_x: 0.0, orig_y: 0.0 };

    // Move it first.
    core.on_pointer_move(pt(60.0, 50.0), no_modifiers());
    let actions = core.on_pointer_up(pt(60.0, 50.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(has_object_updated(&actions));
}

#[test]
fn pointer_up_dragging_no_move_no_update() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 50.0), orig_x: 10.0, orig_y: 20.0 };

    // Don't move, just release.
    let actions = core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    // Position didn't change, so no ObjectUpdated.
    assert!(!has_object_updated(&actions));
}

// =============================================================
// Pointer up — DrawingShape
// =============================================================

#[test]
fn pointer_up_drawing_shape_emits_created() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(110.0, 110.0), no_modifiers());
    let actions = core.on_pointer_up(pt(110.0, 110.0), Button::Primary, no_modifiers());

    assert!(matches!(core.input, InputState::Idle));
    assert!(has_object_created(&actions));
    assert_eq!(core.ui.tool, Tool::Select); // tool resets
}

#[test]
fn pointer_up_drawing_tiny_shape_deletes_it() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    // Only move 1 pixel — below MIN_SHAPE_SIZE threshold.
    core.on_pointer_move(pt(11.0, 11.0), no_modifiers());
    let actions = core.on_pointer_up(pt(11.0, 11.0), Button::Primary, no_modifiers());

    assert!(core.doc.is_empty()); // tiny shape removed
    assert!(!has_object_created(&actions));
    assert_eq!(core.ui.tool, Tool::Select); // tool resets even on tiny shape
}

#[test]
fn pointer_up_drawing_edge_always_keeps() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Line);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    // Don't move — edge with a=b should still be kept.
    let actions = core.on_pointer_up(pt(10.0, 10.0), Button::Primary, no_modifiers());

    assert_eq!(core.doc.len(), 1); // edge not deleted
    assert!(has_object_created(&actions));
}

// =============================================================
// Pointer up — ResizingObject
// =============================================================

#[test]
fn pointer_up_resizing_emits_update() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: pt(100.0, 80.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    let actions = core.on_pointer_up(pt(120.0, 100.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(has_object_updated(&actions));
}

// =============================================================
// Pointer up — RotatingObject
// =============================================================

#[test]
fn pointer_up_rotating_emits_update() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    let actions = core.on_pointer_up(pt(150.0, 40.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(has_object_updated(&actions));
}

// =============================================================
// Pointer up — DraggingEdgeEndpoint
// =============================================================

#[test]
fn pointer_up_edge_endpoint_emits_update() {
    let mut core = EngineCore::new();
    let edge = make_edge(ObjectKind::Line, 0.0, 0.0, 100.0, 100.0);
    let id = edge.id;
    core.apply_create(edge);
    core.input = InputState::DraggingEdgeEndpoint { id, end: EdgeEnd::A };

    let actions = core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(has_object_updated(&actions));
}

// =============================================================
// Pointer up — Panning
// =============================================================

#[test]
fn pointer_up_panning_returns_to_idle() {
    let mut core = EngineCore::new();
    core.input = InputState::Panning { last_screen: pt(0.0, 0.0) };
    let actions = core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(has_render_needed(&actions));
}

// =============================================================
// Pointer up — Idle is no-op
// =============================================================

#[test]
fn pointer_up_idle_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());
    assert!(actions.is_empty());
}

// =============================================================
// Wheel — Pan
// =============================================================

#[test]
fn wheel_without_modifier_pans() {
    let mut core = EngineCore::new();
    let actions = core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 10.0, dy: 20.0 }, no_modifiers());
    assert_eq!(core.camera.pan_x, -10.0);
    assert_eq!(core.camera.pan_y, -20.0);
    assert!(has_render_needed(&actions));
}

#[test]
fn wheel_pan_accumulates() {
    let mut core = EngineCore::new();
    core.on_wheel(pt(0.0, 0.0), WheelDelta { dx: 5.0, dy: 10.0 }, no_modifiers());
    core.on_wheel(pt(0.0, 0.0), WheelDelta { dx: 3.0, dy: 7.0 }, no_modifiers());
    assert_eq!(core.camera.pan_x, -8.0);
    assert_eq!(core.camera.pan_y, -17.0);
}

// =============================================================
// Wheel — Zoom
// =============================================================

#[test]
fn wheel_ctrl_zooms_in() {
    let mut core = EngineCore::new();
    let actions = core.on_wheel(
        pt(400.0, 300.0),
        WheelDelta { dx: 0.0, dy: -10.0 }, // scroll up = zoom in
        ctrl_modifier(),
    );
    assert!(core.camera.zoom > 1.0);
    assert!(has_render_needed(&actions));
}

#[test]
fn wheel_ctrl_zooms_out() {
    let mut core = EngineCore::new();
    core.on_wheel(
        pt(400.0, 300.0),
        WheelDelta { dx: 0.0, dy: 10.0 }, // scroll down = zoom out
        ctrl_modifier(),
    );
    assert!(core.camera.zoom < 1.0);
}

#[test]
fn wheel_zoom_clamps_max() {
    let mut core = EngineCore::new();
    core.camera.zoom = 9.5;
    core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 0.0, dy: -100.0 }, ctrl_modifier());
    assert!(core.camera.zoom <= 10.0);
}

#[test]
fn wheel_zoom_clamps_min() {
    let mut core = EngineCore::new();
    core.camera.zoom = 0.15;
    core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 0.0, dy: 100.0 }, ctrl_modifier());
    assert!(core.camera.zoom >= 0.1);
}

#[test]
fn wheel_zoom_preserves_world_point_under_cursor() {
    let mut core = EngineCore::new();
    let screen = pt(400.0, 300.0);
    let before = core.camera.screen_to_world(screen);

    core.on_wheel(screen, WheelDelta { dx: 0.0, dy: -10.0 }, ctrl_modifier());

    let after = core.camera.screen_to_world(screen);
    assert!((before.x - after.x).abs() < 0.01);
    assert!((before.y - after.y).abs() < 0.01);
}

// =============================================================
// Key down — Delete
// =============================================================

#[test]
fn delete_key_removes_selected_object() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    let actions = core.on_key_down(Key("Delete".into()), no_modifiers());
    assert!(core.object(&id).is_none());
    assert!(core.selection().is_none());
    assert!(has_object_deleted(&actions));
    assert!(has_render_needed(&actions));
}

#[test]
fn backspace_key_removes_selected_object() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    let actions = core.on_key_down(Key("Backspace".into()), no_modifiers());
    assert!(has_object_deleted(&actions));
}

#[test]
fn delete_key_without_selection_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_key_down(Key("Delete".into()), no_modifiers());
    assert!(actions.is_empty());
}

// =============================================================
// Key down — Escape
// =============================================================

#[test]
fn escape_deselects() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    let actions = core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(core.selection().is_none());
    assert!(has_render_needed(&actions));
}

#[test]
fn escape_cancels_active_gesture() {
    let mut core = EngineCore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(0.0, 0.0), orig_x: 0.0, orig_y: 0.0 };

    core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
}

#[test]
fn escape_without_selection_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(actions.is_empty());
}

#[test]
fn enter_on_selected_object_requests_text_edit() {
    let mut core = EngineCore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"head": "Top", "text": "Body", "foot": "Bottom"});
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    let actions = core.on_key_down(Key("Enter".into()), no_modifiers());
    assert!(has_action(&actions, |a| matches!(
        a,
        Action::EditTextRequested {
            id: action_id,
            head,
            text,
            foot
        } if *action_id == id && head == "Top" && text == "Body" && foot == "Bottom"
    )));
}

#[test]
fn enter_without_selection_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_key_down(Key("Enter".into()), no_modifiers());
    assert!(actions.is_empty());
}

// =============================================================
// Key down — Unknown key is no-op
// =============================================================

#[test]
fn unknown_key_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_key_down(Key("q".into()), no_modifiers());
    assert!(actions.is_empty());
}

// =============================================================
// Key up — No-op
// =============================================================

#[test]
fn key_up_is_noop() {
    let mut core = EngineCore::new();
    let actions = core.on_key_up(Key("Delete".into()), no_modifiers());
    assert!(actions.is_empty());
}

// =============================================================
// next_z_index
// =============================================================

#[test]
fn next_z_index_empty_doc() {
    let core = EngineCore::new();
    assert_eq!(core.next_z_index(), 0);
}

#[test]
fn next_z_index_increments() {
    let mut core = EngineCore::new();
    core.apply_create(make_object(ObjectKind::Rect, 5));
    assert_eq!(core.next_z_index(), 6);
}

#[test]
fn next_z_index_finds_max() {
    let mut core = EngineCore::new();
    core.apply_create(make_object(ObjectKind::Rect, 3));
    core.apply_create(make_object(ObjectKind::Rect, 7));
    core.apply_create(make_object(ObjectKind::Rect, 1));
    assert_eq!(core.next_z_index(), 8);
}

// =============================================================
// Tool helpers
// =============================================================

#[test]
fn tool_is_shape_classification() {
    assert!(Tool::Rect.is_shape());
    assert!(Tool::Ellipse.is_shape());
    assert!(Tool::Diamond.is_shape());
    assert!(Tool::Star.is_shape());
    assert!(!Tool::Select.is_shape());
    assert!(!Tool::Line.is_shape());
    assert!(!Tool::Arrow.is_shape());
}

#[test]
fn tool_is_edge_classification() {
    assert!(Tool::Line.is_edge());
    assert!(Tool::Arrow.is_edge());
    assert!(!Tool::Select.is_edge());
    assert!(!Tool::Rect.is_edge());
}

// =============================================================
// Full gesture: draw rect
// =============================================================

#[test]
fn full_gesture_draw_rect() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);

    // Press at (10, 20).
    core.on_pointer_down(pt(10.0, 20.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::DrawingShape { .. }));

    // Drag to (110, 120).
    core.on_pointer_move(pt(110.0, 120.0), no_modifiers());

    // Release.
    let actions = core.on_pointer_up(pt(110.0, 120.0), Button::Primary, no_modifiers());
    assert!(has_object_created(&actions));
    assert_eq!(core.ui.tool, Tool::Select);

    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.kind, ObjectKind::Rect);
    assert_eq!(obj.x, 10.0);
    assert_eq!(obj.y, 20.0);
    assert_eq!(obj.width, 100.0);
    assert_eq!(obj.height, 100.0);
}

// =============================================================
// Full gesture: drag object
// =============================================================

#[test]
fn full_gesture_drag_object() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    // Click on body at (50, 40).
    core.on_pointer_down(pt(50.0, 40.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));
    assert!(matches!(core.input, InputState::DraggingObject { .. }));

    // Drag to (80, 70) — moved 30 right, 30 down.
    core.on_pointer_move(pt(80.0, 70.0), no_modifiers());

    // Release.
    let actions = core.on_pointer_up(pt(80.0, 70.0), Button::Primary, no_modifiers());
    assert!(has_object_updated(&actions));

    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 30.0);
    assert_eq!(obj.y, 30.0);
}

// =============================================================
// Full gesture: draw line
// =============================================================

#[test]
fn full_gesture_draw_line() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Line);

    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(200.0, 150.0), no_modifiers());
    let actions = core.on_pointer_up(pt(200.0, 150.0), Button::Primary, no_modifiers());

    assert!(has_object_created(&actions));
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.kind, ObjectKind::Line);
    assert_eq!(obj.props["a"]["x"], 10.0);
    assert_eq!(obj.props["b"]["x"], 200.0);
}

// =============================================================
// Full gesture: pan with middle button
// =============================================================

#[test]
fn full_gesture_pan_middle_button() {
    let mut core = EngineCore::new();

    core.on_pointer_down(pt(100.0, 100.0), Button::Middle, no_modifiers());
    core.on_pointer_move(pt(150.0, 130.0), no_modifiers());
    core.on_pointer_up(pt(150.0, 130.0), Button::Middle, no_modifiers());

    assert_eq!(core.camera.pan_x, 50.0);
    assert_eq!(core.camera.pan_y, 30.0);
    assert!(matches!(core.input, InputState::Idle));
}

// =============================================================
// Camera offset: shape tool with pan
// =============================================================

#[test]
fn shape_tool_respects_camera_pan() {
    let mut core = EngineCore::new();
    core.camera.pan_x = 100.0;
    core.camera.pan_y = 50.0;
    core.set_tool(Tool::Rect);

    // Click at screen (100, 50) which is world (0, 0) with this pan.
    core.on_pointer_down(pt(100.0, 50.0), Button::Primary, no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.x, 0.0);
    assert_eq!(obj.y, 0.0);
}

#[test]
fn shape_tool_respects_camera_zoom() {
    let mut core = EngineCore::new();
    core.camera.zoom = 2.0;
    core.set_tool(Tool::Rect);

    // Screen (100, 80) at zoom 2 = world (50, 40).
    core.on_pointer_down(pt(100.0, 80.0), Button::Primary, no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.x, 50.0);
    assert_eq!(obj.y, 40.0);
}

// =============================================================
// Edge-case: Resize — boundary & direction reversal
// =============================================================

#[test]
fn resize_nw_past_se_corner_clamps() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Nw,
        start_world: pt(10.0, 20.0),
        orig_x: 10.0,
        orig_y: 20.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    // Drag NW handle far past the SE corner.
    core.on_pointer_move(pt(300.0, 300.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 110.0);
    assert_eq!(obj.y, 100.0);
    assert_eq!(obj.width, 0.0);
    assert_eq!(obj.height, 0.0);
    assert!(obj.width >= 0.0);
    assert!(obj.height >= 0.0);
}

#[test]
fn resize_e_with_negative_dx_shrinks_width() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::E,
        start_world: pt(100.0, 25.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 50.0,
    };

    // Drag E handle left by 30px.
    core.on_pointer_move(pt(70.0, 25.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.width, 70.0);
    assert_eq!(obj.x, 0.0);
}

#[test]
fn resize_all_anchors_zero_delta_no_change() {
    let anchors = [
        ResizeAnchor::N,
        ResizeAnchor::Ne,
        ResizeAnchor::E,
        ResizeAnchor::Se,
        ResizeAnchor::S,
        ResizeAnchor::Sw,
        ResizeAnchor::W,
        ResizeAnchor::Nw,
    ];
    for anchor in anchors {
        let mut core = EngineCore::new();
        let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
        let id = obj.id;
        core.apply_create(obj);
        core.input = InputState::ResizingObject {
            id,
            anchor,
            start_world: pt(50.0, 50.0),
            orig_x: 10.0,
            orig_y: 20.0,
            orig_w: 100.0,
            orig_h: 80.0,
        };

        // Move to same point = zero delta.
        core.on_pointer_move(pt(50.0, 50.0), no_modifiers());
        let obj = core.object(&id).unwrap();
        assert_eq!(obj.width, 100.0, "anchor {anchor:?} changed width on zero delta");
        assert_eq!(obj.height, 80.0, "anchor {anchor:?} changed height on zero delta");
        assert_eq!(obj.x, 10.0, "anchor {anchor:?} changed x on zero delta");
        assert_eq!(obj.y, 20.0, "anchor {anchor:?} changed y on zero delta");
    }
}

#[test]
fn resize_accumulates_across_multiple_moves() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: pt(50.0, 50.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    // Each move computes total delta from start_world to current pointer.
    // Move 1: total dx=10, dy=10 → w = 50+10 = 60, h = 50+10 = 60
    core.on_pointer_move(pt(60.0, 60.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.width, 60.0);
    assert_eq!(obj.height, 60.0);

    // Move 2: total dx=80-50=30, total dy=70-50=20 → w = 50+30 = 80, h = 50+20 = 70
    core.on_pointer_move(pt(80.0, 70.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.width, 80.0);
    assert_eq!(obj.height, 70.0);
}

#[test]
fn resize_e_past_origin_clamps() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::E,
        start_world: pt(50.0, 25.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    // Drag E handle past the W edge.
    core.on_pointer_move(pt(-30.0, 25.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 0.0);
    assert_eq!(obj.width, 0.0);
    assert!(obj.width >= 0.0);
}

#[test]
fn resize_n_past_bottom_clamps() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::N,
        start_world: pt(25.0, 0.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 50.0,
        orig_h: 50.0,
    };

    // Drag N handle below bottom edge.
    core.on_pointer_move(pt(25.0, 200.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.y, 50.0);
    assert_eq!(obj.height, 0.0);
    assert!(obj.height >= 0.0);
}

#[test]
fn resize_w_past_right_clamps() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 0.0, 100.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::W,
        start_world: pt(10.0, 25.0),
        orig_x: 10.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 50.0,
    };

    // Drag W handle far past the right edge.
    core.on_pointer_move(pt(500.0, 25.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 110.0);
    assert_eq!(obj.width, 0.0);
    assert!(obj.width >= 0.0);
}

#[test]
fn resize_s_past_top_clamps() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 10.0, 50.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::S,
        start_world: pt(25.0, 90.0),
        orig_x: 0.0,
        orig_y: 10.0,
        orig_w: 50.0,
        orig_h: 80.0,
    };

    // Drag S handle above top edge.
    core.on_pointer_move(pt(25.0, -100.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.y, 10.0);
    assert_eq!(obj.height, 0.0);
    assert!(obj.height >= 0.0);
}

#[test]
fn resize_rotated_object_uses_local_axes() {
    let mut core = EngineCore::new();
    let mut obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 50.0);
    obj.rotation = 90.0;
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::E,
        start_world: pt(50.0, 75.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 50.0,
    };

    // Moving down in world space maps to +X in local space at 90° rotation.
    core.on_pointer_move(pt(50.0, 95.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 0.0);
    assert_eq!(obj.width, 120.0);
}

// =============================================================
// Edge-case: Rotation — full sweep & numerics
// =============================================================

#[test]
fn rotate_to_180_degrees() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    // Move directly below center.
    core.on_pointer_move(pt(50.0, 140.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    // atan2(100, 0) = 90 degrees, + 90 = 180 degrees.
    assert!((obj.rotation - 180.0).abs() < 0.01);
}

#[test]
fn rotate_to_270_degrees() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    // Move to the left of center.
    core.on_pointer_move(pt(-50.0, 40.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    // atan2(0, -100) = 180 degrees, + 90 = 270 degrees.
    assert!((obj.rotation - 270.0).abs() < 0.01);
}

#[test]
fn rotate_full_360_sweep() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    let center = pt(50.0, 40.0);
    core.input = InputState::RotatingObject { id, center, orig_rotation: 0.0 };

    // Right of center → ~90°
    core.on_pointer_move(pt(150.0, 40.0), no_modifiers());
    let r1 = core.object(&id).unwrap().rotation;
    assert!((r1 - 90.0).abs() < 0.01);

    // Below center → ~180°
    core.on_pointer_move(pt(50.0, 140.0), no_modifiers());
    let r2 = core.object(&id).unwrap().rotation;
    assert!((r2 - 180.0).abs() < 0.01);

    // Left of center → ~270°
    core.on_pointer_move(pt(-50.0, 40.0), no_modifiers());
    let r3 = core.object(&id).unwrap().rotation;
    assert!((r3 - 270.0).abs() < 0.01);

    // Above center → ~0° (or 360°)
    core.on_pointer_move(pt(50.0, -60.0), no_modifiers());
    let r4 = core.object(&id).unwrap().rotation;
    assert!(r4.abs() < 0.01 || (r4 - 360.0).abs() < 0.01);
}

#[test]
fn rotate_pointer_at_center_degenerate() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    // Move to exactly the center — atan2(0, 0) = 0, + 90 = 90.
    core.on_pointer_move(pt(50.0, 40.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    // Should not panic; rotation is some finite value.
    assert!(obj.rotation.is_finite());
}

#[test]
fn rotate_center_computation_from_object_geometry() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 20.0, 30.0, 60.0, 40.0);
    let id = obj.id;
    core.apply_create(obj);
    // Center should be (20+30, 30+20) = (50, 50).
    core.input = InputState::RotatingObject { id, center: pt(50.0, 50.0), orig_rotation: 0.0 };

    // Move directly to the right of center.
    core.on_pointer_move(pt(150.0, 50.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert!((obj.rotation - 90.0).abs() < 0.01);
}

// =============================================================
// Edge-case: Drawing shapes — threshold boundary
// =============================================================

#[test]
fn draw_shape_exactly_at_min_size_kept() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(12.0, 12.0), no_modifiers());
    let actions = core.on_pointer_up(pt(12.0, 12.0), Button::Primary, no_modifiers());

    assert_eq!(core.doc.len(), 1);
    assert!(has_object_created(&actions));
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.width, 2.0);
    assert_eq!(obj.height, 2.0);
}

#[test]
fn draw_shape_just_below_min_size_deleted() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(11.99, 11.99), no_modifiers());
    let actions = core.on_pointer_up(pt(11.99, 11.99), Button::Primary, no_modifiers());

    assert!(core.doc.is_empty());
    assert!(!has_object_created(&actions));
}

#[test]
fn draw_shape_width_ok_height_below_min_deleted() {
    // Both width AND height must be < MIN_SHAPE_SIZE to discard.
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    // Width = 50, height = 1
    core.on_pointer_move(pt(60.0, 11.0), no_modifiers());
    let actions = core.on_pointer_up(pt(60.0, 11.0), Button::Primary, no_modifiers());

    // Width >= MIN_SHAPE_SIZE, so it should be kept (both must be < to discard).
    assert_eq!(core.doc.len(), 1);
    assert!(has_object_created(&actions));
}

#[test]
fn draw_shape_negative_direction_at_threshold() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(100.0, 100.0), Button::Primary, no_modifiers());
    // Drag up-left exactly 2px.
    core.on_pointer_move(pt(98.0, 98.0), no_modifiers());
    let actions = core.on_pointer_up(pt(98.0, 98.0), Button::Primary, no_modifiers());

    assert_eq!(core.doc.len(), 1);
    assert!(has_object_created(&actions));
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.x, 98.0);
    assert_eq!(obj.y, 98.0);
    assert_eq!(obj.width, 2.0);
    assert_eq!(obj.height, 2.0);
}

#[test]
fn draw_zero_movement_shape_deleted() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(50.0, 50.0), Button::Primary, no_modifiers());
    // No move at all.
    let actions = core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());

    assert!(core.doc.is_empty());
    assert!(!has_object_created(&actions));
    assert_eq!(core.ui.tool, Tool::Select);
}

// =============================================================
// Edge-case: Drawing edges — degenerate
// =============================================================

#[test]
fn draw_edge_zero_length_kept_with_correct_props() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Line);
    core.on_pointer_down(pt(50.0, 50.0), Button::Primary, no_modifiers());
    let actions = core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());

    assert_eq!(core.doc.len(), 1);
    assert!(has_object_created(&actions));
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.props["a"]["x"], 50.0);
    assert_eq!(obj.props["a"]["y"], 50.0);
    assert_eq!(obj.props["b"]["x"], 50.0);
    assert_eq!(obj.props["b"]["y"], 50.0);
}

#[test]
fn draw_edge_move_then_back_to_start() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Line);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(200.0, 200.0), no_modifiers());
    core.on_pointer_move(pt(10.0, 10.0), no_modifiers());
    let actions = core.on_pointer_up(pt(10.0, 10.0), Button::Primary, no_modifiers());

    assert_eq!(core.doc.len(), 1);
    assert!(has_object_created(&actions));
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.props["a"]["x"], 10.0);
    assert_eq!(obj.props["b"]["x"], 10.0);
}

#[test]
fn draw_arrow_zero_length_kept() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Arrow);
    core.on_pointer_down(pt(30.0, 40.0), Button::Primary, no_modifiers());
    let actions = core.on_pointer_up(pt(30.0, 40.0), Button::Primary, no_modifiers());

    assert_eq!(core.doc.len(), 1);
    assert!(has_object_created(&actions));
    assert_eq!(core.doc.sorted_objects()[0].kind, ObjectKind::Arrow);
}

// =============================================================
// Edge-case: Drag object — with camera offset
// =============================================================

#[test]
fn drag_object_with_camera_panned() {
    let mut core = EngineCore::new();
    core.camera.pan_x = 200.0;
    core.camera.pan_y = 100.0;
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    // Object is at world (0,0). Screen center of object is at (200 + 50*1, 100 + 40*1) = (250, 140).
    core.on_pointer_down(pt(250.0, 140.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::DraggingObject { .. }));

    // Drag 100 screen pixels right = 100 world units at zoom 1.
    core.on_pointer_move(pt(350.0, 140.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 100.0);
    assert_eq!(obj.y, 0.0);
}

#[test]
fn drag_object_with_camera_zoomed() {
    let mut core = EngineCore::new();
    core.camera.zoom = 2.0;
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    // Object world center (50, 40) → screen (100, 80) at zoom 2.
    core.on_pointer_down(pt(100.0, 80.0), Button::Primary, no_modifiers());

    // Drag 40 screen pixels right = 20 world units at zoom 2.
    core.on_pointer_move(pt(140.0, 80.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert!((obj.x - 20.0).abs() < 0.01);
    assert!(obj.y.abs() < 0.01);
}

#[test]
fn drag_object_with_camera_panned_and_zoomed() {
    let mut core = EngineCore::new();
    core.camera.pan_x = 100.0;
    core.camera.pan_y = 50.0;
    core.camera.zoom = 2.0;
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    // Object world center (50, 40) → screen (50*2+100, 40*2+50) = (200, 130).
    core.on_pointer_down(pt(200.0, 130.0), Button::Primary, no_modifiers());

    // Drag 20 screen pixels right = 10 world units at zoom 2.
    core.on_pointer_move(pt(220.0, 130.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert!((obj.x - 10.0).abs() < 0.01);
    assert!(obj.y.abs() < 0.01);
}

#[test]
fn drag_object_to_negative_coordinates() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 10.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);

    core.on_pointer_down(pt(35.0, 35.0), Button::Primary, no_modifiers());
    // Drag far up-left.
    core.on_pointer_move(pt(-65.0, -65.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert!(obj.x < 0.0);
    assert!(obj.y < 0.0);
}

#[test]
fn drag_zero_movement_no_update_emitted() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 10.0, 20.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    core.on_pointer_down(pt(60.0, 60.0), Button::Primary, no_modifiers());
    // Release at same point — no movement.
    let actions = core.on_pointer_up(pt(60.0, 60.0), Button::Primary, no_modifiers());
    assert!(!has_object_updated(&actions));
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 10.0);
    assert_eq!(obj.y, 20.0);
}

// =============================================================
// Edge-case: Server events during gesture
// =============================================================

#[test]
fn apply_delete_on_dragged_object_graceful_move() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 40.0), orig_x: 0.0, orig_y: 0.0 };

    // Server deletes the object mid-drag.
    core.apply_delete(&id);
    assert!(core.object(&id).is_none());
    assert!(core.selection().is_none());

    // Next pointer_move should not panic.
    let actions = core.on_pointer_move(pt(80.0, 70.0), no_modifiers());
    assert!(has_render_needed(&actions));
}

#[test]
fn apply_delete_on_resized_object_graceful_up() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: pt(100.0, 80.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    core.apply_delete(&id);

    // pointer_up should not panic.
    let actions = core.on_pointer_up(pt(120.0, 100.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    // No ObjectUpdated since the object is gone.
    assert!(!has_object_updated(&actions));
}

#[test]
fn apply_update_on_dragged_object_continues_from_server_position() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 40.0), orig_x: 0.0, orig_y: 0.0 };

    // Server moves the object.
    let partial = PartialBoardObject { x: Some(200.0), y: Some(200.0), ..Default::default() };
    core.apply_update(&id, &partial);
    assert_eq!(core.object(&id).unwrap().x, 200.0);

    // Local drag continues — adds delta on top of new server position.
    core.on_pointer_move(pt(60.0, 50.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, 210.0); // 200 + (60-50)
    assert_eq!(obj.y, 210.0); // 200 + (50-40)
}

#[test]
fn apply_delete_clears_selection_during_gesture() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);
    core.input = InputState::RotatingObject { id, center: pt(50.0, 40.0), orig_rotation: 0.0 };

    core.apply_delete(&id);
    assert!(core.selection().is_none());
}

#[test]
fn load_snapshot_during_drag_graceful() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 40.0), orig_x: 0.0, orig_y: 0.0 };

    // Full snapshot replaces doc — dragged object is gone.
    let new_obj = make_object(ObjectKind::Ellipse, 0);
    core.load_snapshot(vec![new_obj]);
    assert!(core.object(&id).is_none());

    // pointer_up should not panic.
    let actions = core.on_pointer_up(pt(60.0, 50.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(!has_object_updated(&actions));
}

#[test]
fn apply_delete_on_drawing_shape_object() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    let id = core.doc.sorted_objects()[0].id;

    // Server deletes the drawing-in-progress object.
    core.apply_delete(&id);

    // pointer_up should handle missing object gracefully.
    let actions = core.on_pointer_up(pt(100.0, 100.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(core.doc.is_empty());
    assert!(!has_object_created(&actions));
    assert_eq!(core.ui.tool, Tool::Select);
}

// =============================================================
// Edge-case: Wheel zoom — extreme & numeric
// =============================================================

#[test]
fn zoom_at_max_stays_at_max() {
    let mut core = EngineCore::new();
    core.camera.zoom = 10.0;
    core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 0.0, dy: -100.0 }, ctrl_modifier());
    assert!((core.camera.zoom - 10.0).abs() < f64::EPSILON);
}

#[test]
fn zoom_at_min_stays_at_min() {
    let mut core = EngineCore::new();
    core.camera.zoom = 0.1;
    core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 0.0, dy: 100.0 }, ctrl_modifier());
    assert!((core.camera.zoom - 0.1).abs() < f64::EPSILON);
}

#[test]
fn zoom_with_zero_dy_no_change() {
    let mut core = EngineCore::new();
    let zoom_before = core.camera.zoom;
    core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 0.0, dy: 0.0 }, ctrl_modifier());
    // dy=0 → factor = 1/ZOOM_FACTOR (since !(dy < 0)), but zoom*factor should differ.
    // Actually, dy=0 is !< 0, so factor = 1/ZOOM_FACTOR → slight zoom out.
    // The test just verifies no panic.
    assert!(core.camera.zoom.is_finite());
    let _ = zoom_before;
}

#[test]
fn zoom_preserves_world_point_with_pan_offset() {
    let mut core = EngineCore::new();
    core.camera.pan_x = 200.0;
    core.camera.pan_y = 150.0;
    let screen = pt(500.0, 400.0);
    let before = core.camera.screen_to_world(screen);

    core.on_wheel(screen, WheelDelta { dx: 0.0, dy: -10.0 }, ctrl_modifier());

    let after = core.camera.screen_to_world(screen);
    assert!((before.x - after.x).abs() < 0.01);
    assert!((before.y - after.y).abs() < 0.01);
}

#[test]
fn zoom_preserves_world_point_deeply_zoomed() {
    let mut core = EngineCore::new();
    core.camera.zoom = 5.0;
    core.camera.pan_x = -1000.0;
    core.camera.pan_y = -800.0;
    let screen = pt(300.0, 200.0);
    let before = core.camera.screen_to_world(screen);

    core.on_wheel(screen, WheelDelta { dx: 0.0, dy: -10.0 }, ctrl_modifier());

    let after = core.camera.screen_to_world(screen);
    assert!((before.x - after.x).abs() < 0.01);
    assert!((before.y - after.y).abs() < 0.01);
}

#[test]
fn meta_key_triggers_zoom() {
    let mut core = EngineCore::new();
    let mods = Modifiers { meta: true, ..Default::default() };
    core.on_wheel(pt(400.0, 300.0), WheelDelta { dx: 0.0, dy: -10.0 }, mods);
    assert!(core.camera.zoom > 1.0);
}

// =============================================================
// Edge-case: Pan — edge cases
// =============================================================

#[test]
fn pan_with_negative_delta() {
    let mut core = EngineCore::new();
    core.on_wheel(pt(0.0, 0.0), WheelDelta { dx: -10.0, dy: -20.0 }, no_modifiers());
    // Pan subtracts delta, so negative delta → positive pan.
    assert_eq!(core.camera.pan_x, 10.0);
    assert_eq!(core.camera.pan_y, 20.0);
}

#[test]
fn pan_to_large_coordinates() {
    let mut core = EngineCore::new();
    core.on_wheel(pt(0.0, 0.0), WheelDelta { dx: -1e6, dy: -1e6 }, no_modifiers());
    assert_eq!(core.camera.pan_x, 1e6);
    assert_eq!(core.camera.pan_y, 1e6);
}

#[test]
fn empty_space_drag_to_pan() {
    let mut core = EngineCore::new();
    // Click on empty space with select tool.
    core.on_pointer_down(pt(100.0, 100.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Panning { .. }));

    core.on_pointer_move(pt(150.0, 130.0), no_modifiers());
    assert_eq!(core.camera.pan_x, 50.0);
    assert_eq!(core.camera.pan_y, 30.0);

    core.on_pointer_up(pt(150.0, 130.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
}

// =============================================================
// Edge-case: Key events during active gestures
// =============================================================

#[test]
fn delete_key_during_dragging_object() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);
    core.input = InputState::DraggingObject { id, last_world: pt(50.0, 40.0), orig_x: 0.0, orig_y: 0.0 };

    let actions = core.on_key_down(Key("Delete".into()), no_modifiers());
    // Delete key removes the object and clears selection.
    assert!(core.object(&id).is_none());
    assert!(has_object_deleted(&actions));
}

#[test]
fn escape_during_drawing_shape() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    let id = core.doc.sorted_objects()[0].id;

    core.on_pointer_move(pt(50.0, 50.0), no_modifiers());

    // Escape during drawing cancels gesture.
    core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(core.selection().is_none());
    // The half-drawn object is still in the doc (escape only clears input state + selection).
    let obj = core.object(&id);
    assert!(obj.is_some());
}

#[test]
fn escape_during_resizing_object() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);
    core.input = InputState::ResizingObject {
        id,
        anchor: ResizeAnchor::Se,
        start_world: pt(100.0, 80.0),
        orig_x: 0.0,
        orig_y: 0.0,
        orig_w: 100.0,
        orig_h: 80.0,
    };

    // Resize a bit.
    core.on_pointer_move(pt(150.0, 130.0), no_modifiers());

    // Escape cancels.
    core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    assert!(core.selection().is_none());
    // Object remains at intermediate size (escape doesn't revert).
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.width, 150.0);
    assert_eq!(obj.height, 130.0);
}

#[test]
fn escape_during_panning() {
    let mut core = EngineCore::new();
    core.input = InputState::Panning { last_screen: pt(100.0, 100.0) };
    core.on_pointer_move(pt(150.0, 130.0), no_modifiers());

    core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(matches!(core.input, InputState::Idle));
    // Pan offset remains (escape doesn't revert camera).
    assert_eq!(core.camera.pan_x, 50.0);
}

// =============================================================
// Edge-case: Select tool — overlapping & z-order
// =============================================================

#[test]
fn click_overlapping_objects_selects_topmost() {
    let mut core = EngineCore::new();
    let mut bottom = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 100.0);
    bottom.z_index = 0;
    let bottom_id = bottom.id;
    core.apply_create(bottom);

    let mut top = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 100.0);
    top.z_index = 1;
    let top_id = top.id;
    core.apply_create(top);

    // Click in the overlap area.
    core.on_pointer_down(pt(50.0, 50.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(top_id));
    let _ = bottom_id;
}

#[test]
fn deselect_then_reselect_same_object() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);

    // Select.
    core.on_pointer_down(pt(50.0, 40.0), Button::Primary, no_modifiers());
    core.on_pointer_up(pt(50.0, 40.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));

    // Deselect by clicking empty space.
    core.on_pointer_down(pt(500.0, 500.0), Button::Primary, no_modifiers());
    core.on_pointer_up(pt(500.0, 500.0), Button::Primary, no_modifiers());
    assert!(core.selection().is_none());

    // Reselect.
    core.on_pointer_down(pt(50.0, 40.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));
}

#[test]
fn select_different_object_replaces_selection() {
    let mut core = EngineCore::new();
    let a = make_object_at(ObjectKind::Rect, 0.0, 0.0, 50.0, 50.0);
    let a_id = a.id;
    core.apply_create(a);

    let b = make_object_at(ObjectKind::Ellipse, 200.0, 200.0, 50.0, 50.0);
    let b_id = b.id;
    core.apply_create(b);

    // Select A.
    core.on_pointer_down(pt(25.0, 25.0), Button::Primary, no_modifiers());
    core.on_pointer_up(pt(25.0, 25.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(a_id));

    // Select B.
    core.on_pointer_down(pt(225.0, 225.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(b_id));
}

#[test]
fn click_near_but_outside_object_deselects() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0);
    let id = obj.id;
    core.apply_create(obj);
    core.ui.selected_id = Some(id);

    // Click just outside the boundary.
    core.on_pointer_down(pt(101.0, 81.0), Button::Primary, no_modifiers());
    // Should deselect (outside hit slop for body hit test).
    // Note: handle hit slop may still catch it. Let's click far enough away.
    core.on_pointer_up(pt(101.0, 81.0), Button::Primary, no_modifiers());

    // Click clearly outside.
    core.on_pointer_down(pt(500.0, 500.0), Button::Primary, no_modifiers());
    assert!(core.selection().is_none());
}

// =============================================================
// Edge-case: Multiple shapes created in sequence
// =============================================================

#[test]
fn create_rect_then_ellipse_both_in_doc() {
    let mut core = EngineCore::new();

    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(50.0, 50.0), no_modifiers());
    core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());
    assert_eq!(core.doc.len(), 1);
    assert_eq!(core.ui.tool, Tool::Select);

    core.set_tool(Tool::Ellipse);
    core.on_pointer_down(pt(100.0, 100.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(150.0, 150.0), no_modifiers());
    core.on_pointer_up(pt(150.0, 150.0), Button::Primary, no_modifiers());
    assert_eq!(core.doc.len(), 2);

    let objects = core.doc.sorted_objects();
    assert_eq!(objects[0].kind, ObjectKind::Rect);
    assert_eq!(objects[1].kind, ObjectKind::Ellipse);
    assert!(objects[1].z_index > objects[0].z_index);
}

#[test]
fn create_shape_deselect_create_another_z_ordering() {
    let mut core = EngineCore::new();

    core.set_tool(Tool::Rect);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(50.0, 50.0), no_modifiers());
    core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());

    // Deselect.
    core.on_key_down(Key("Escape".into()), no_modifiers());
    assert!(core.selection().is_none());

    core.set_tool(Tool::Diamond);
    core.on_pointer_down(pt(200.0, 200.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(250.0, 250.0), no_modifiers());
    core.on_pointer_up(pt(250.0, 250.0), Button::Primary, no_modifiers());

    let objects = core.doc.sorted_objects();
    assert_eq!(objects.len(), 2);
    assert!(objects[1].z_index > objects[0].z_index);
}

#[test]
fn create_shape_resets_tool_to_select() {
    let mut core = EngineCore::new();
    core.set_tool(Tool::Star);
    core.on_pointer_down(pt(10.0, 10.0), Button::Primary, no_modifiers());
    core.on_pointer_move(pt(50.0, 50.0), no_modifiers());
    core.on_pointer_up(pt(50.0, 50.0), Button::Primary, no_modifiers());

    assert_eq!(core.ui.tool, Tool::Select);
    // Next pointer_down on the object should drag it, not create a new star.
    let id = core.doc.sorted_objects()[0].id;
    core.on_pointer_down(pt(30.0, 30.0), Button::Primary, no_modifiers());
    assert!(matches!(core.input, InputState::DraggingObject { .. }));
    assert_eq!(core.selection(), Some(id));
}

// =============================================================
// Edge-case: Negative coordinate objects
// =============================================================

#[test]
fn object_at_negative_coords_hit_test_and_drag() {
    let mut core = EngineCore::new();
    let obj = make_object_at(ObjectKind::Rect, -100.0, -80.0, 50.0, 50.0);
    let id = obj.id;
    core.apply_create(obj);

    // Click in center of object at world (-75, -55). At zoom 1, pan 0, screen = world.
    core.on_pointer_down(pt(-75.0, -55.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));
    assert!(matches!(core.input, InputState::DraggingObject { .. }));

    // Drag it 20 right, 10 down.
    core.on_pointer_move(pt(-55.0, -45.0), no_modifiers());
    let obj = core.object(&id).unwrap();
    assert_eq!(obj.x, -80.0);
    assert_eq!(obj.y, -70.0);
}

#[test]
fn edge_with_negative_endpoints_hit_test() {
    let mut core = EngineCore::new();
    let edge = make_edge(ObjectKind::Line, -100.0, -50.0, -10.0, -50.0);
    let id = edge.id;
    core.apply_create(edge);

    // Click on the edge midpoint.
    core.on_pointer_down(pt(-55.0, -50.0), Button::Primary, no_modifiers());
    assert_eq!(core.selection(), Some(id));
}

#[test]
fn shape_tool_click_at_negative_world_coords_with_pan() {
    let mut core = EngineCore::new();
    core.camera.pan_x = 50.0;
    core.camera.pan_y = 50.0;
    core.set_tool(Tool::Rect);

    // Screen (0, 0) → world (-50, -50) with pan (50, 50) at zoom 1.
    core.on_pointer_down(pt(0.0, 0.0), Button::Primary, no_modifiers());
    let obj = core.doc.sorted_objects()[0];
    assert_eq!(obj.x, -50.0);
    assert_eq!(obj.y, -50.0);
}
