use super::*;
use crate::net::types::{BoardObject, Frame, FrameStatus};
use crate::state::board::BoardState;

fn frame(syscall: &str, status: FrameStatus, data: serde_json::Value) -> Frame {
    Frame {
        id: "f1".to_owned(),
        parent_id: None,
        ts: 100,
        board_id: Some("b1".to_owned()),
        from: Some("u1".to_owned()),
        syscall: syscall.to_owned(),
        status,
        data,
    }
}

fn obj(id: &str) -> BoardObject {
    BoardObject {
        id: id.to_owned(),
        board_id: "b1".to_owned(),
        kind: "rectangle".to_owned(),
        x: 1.0,
        y: 2.0,
        width: Some(10.0),
        height: Some(20.0),
        rotation: 0.0,
        z_index: 1,
        props: serde_json::json!({}),
        created_by: None,
        version: 1,
        group_id: None,
    }
}

#[test]
fn is_object_related_syscall_recognizes_supported_routes() {
    assert!(is_object_related_syscall("object:create"));
    assert!(is_object_related_syscall("object:update"));
    assert!(is_object_related_syscall("object:drag"));
    assert!(is_object_related_syscall("cursor:moved"));
    assert!(!is_object_related_syscall("board:list"));
}

#[test]
fn apply_object_frame_ignores_invalid_object_create_payload() {
    let mut board = BoardState::default();
    let f = frame("object:create", FrameStatus::Done, serde_json::json!({ "id": "x" }));
    apply_object_frame(&f, &mut board);
    assert!(board.objects.is_empty());
}

#[test]
fn apply_object_frame_delete_removes_drag_and_selection_state() {
    let mut board = BoardState::default();
    board.objects.insert("o1".to_owned(), obj("o1"));
    board.drag_objects.insert("o1".to_owned(), obj("o1"));
    board.drag_updated_at.insert("o1".to_owned(), 5);
    board.selection.insert("o1".to_owned());

    let f = frame("object:delete", FrameStatus::Done, serde_json::json!({ "id": "o1" }));
    apply_object_frame(&f, &mut board);

    assert!(!board.objects.contains_key("o1"));
    assert!(!board.drag_objects.contains_key("o1"));
    assert!(!board.drag_updated_at.contains_key("o1"));
    assert!(!board.selection.contains("o1"));
}

#[test]
fn apply_object_frame_ignores_unknown_syscall() {
    let mut board = BoardState::default();
    board.objects.insert("o1".to_owned(), obj("o1"));
    let before = board.objects.len();
    let f = frame("unknown", FrameStatus::Done, serde_json::json!({}));
    apply_object_frame(&f, &mut board);
    assert_eq!(board.objects.len(), before);
}
