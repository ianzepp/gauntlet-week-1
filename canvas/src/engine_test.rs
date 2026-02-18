use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::doc::{BoardObject, ObjectKind, PartialBoardObject};
use crate::input::Tool;

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
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    core.apply_create(obj);

    core.set_text(&id, String::new(), String::new(), String::new());

    let updated = core.object(&id).unwrap();
    assert_eq!(updated.props["head"], "");
    assert_eq!(updated.props["text"], "");
    assert_eq!(updated.props["foot"], "");
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
