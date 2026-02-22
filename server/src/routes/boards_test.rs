use super::*;
use uuid::Uuid;

#[test]
fn board_error_to_status_maps_not_found() {
    let err = board::BoardError::NotFound(Uuid::nil());
    assert_eq!(board_error_to_status(err), StatusCode::NOT_FOUND);
}

#[test]
fn board_error_to_status_maps_forbidden() {
    let err = board::BoardError::Forbidden(Uuid::nil());
    assert_eq!(board_error_to_status(err), StatusCode::FORBIDDEN);
}

#[test]
fn parse_import_skips_meta_line() {
    let line = r#"{"type":"board_export_meta","version":1,"board_id":"00000000-0000-0000-0000-000000000000"}"#;
    let result = parse_import_object_line(line, Uuid::nil(), Uuid::nil()).unwrap();
    assert!(result.is_none());
}

#[test]
fn parse_import_skips_unknown_type() {
    let line = r#"{"type":"unknown_type","foo":"bar"}"#;
    let result = parse_import_object_line(line, Uuid::nil(), Uuid::nil()).unwrap();
    assert!(result.is_none());
}

#[test]
fn parse_import_parses_object_line() {
    let board_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let line = r#"{"type":"object","kind":"sticky_note","x":100.5,"y":200.0,"width":120.0,"height":80.0,"rotation":45.0,"z_index":3,"props":{"text":"hello"},"version":2}"#;
    let result = parse_import_object_line(line, board_id, user_id)
        .unwrap()
        .unwrap();
    assert_eq!(result.board_id, board_id);
    assert_eq!(result.created_by, Some(user_id));
    assert_eq!(result.kind, "sticky_note");
    assert!((result.x - 100.5).abs() < f64::EPSILON);
    assert!((result.y - 200.0).abs() < f64::EPSILON);
    assert_eq!(result.width, Some(120.0));
    assert_eq!(result.height, Some(80.0));
    assert!((result.rotation - 45.0).abs() < f64::EPSILON);
    assert_eq!(result.z_index, 3);
    assert_eq!(result.version, 2);
    assert_eq!(result.props.get("text").and_then(|v| v.as_str()), Some("hello"));
}

#[test]
fn parse_import_defaults_missing_fields() {
    let line = r#"{"kind":"rectangle"}"#;
    let result = parse_import_object_line(line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    assert_eq!(result.kind, "rectangle");
    assert!((result.x).abs() < f64::EPSILON);
    assert!((result.y).abs() < f64::EPSILON);
    assert!(result.width.is_none());
    assert!(result.height.is_none());
    assert!((result.rotation).abs() < f64::EPSILON);
    assert_eq!(result.z_index, 0);
    assert_eq!(result.version, 1);
    assert!(result.group_id.is_none());
}

#[test]
fn parse_import_defaults_kind_to_rectangle() {
    let line = r#"{"type":"object","x":10}"#;
    let result = parse_import_object_line(line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    assert_eq!(result.kind, "rectangle");
}

#[test]
fn parse_import_handles_float_z_index() {
    let line = r#"{"kind":"ellipse","z_index":5.0}"#;
    let result = parse_import_object_line(line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    assert_eq!(result.z_index, 5);
}

#[test]
fn parse_import_clamps_version_minimum() {
    let line = r#"{"kind":"rectangle","version":0}"#;
    let result = parse_import_object_line(line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    assert_eq!(result.version, 1);
}

#[test]
fn parse_import_parses_group_id() {
    let gid = Uuid::new_v4();
    let line = format!(r#"{{"kind":"rectangle","group_id":"{}"}}"#, gid);
    let result = parse_import_object_line(&line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    assert_eq!(result.group_id, Some(gid));
}

#[test]
fn parse_import_invalid_json_returns_error() {
    let result = parse_import_object_line("not json", Uuid::nil(), Uuid::nil());
    assert!(result.is_err());
}

#[test]
fn parse_import_non_object_json_returns_none() {
    let result = parse_import_object_line("42", Uuid::nil(), Uuid::nil()).unwrap();
    assert!(result.is_none());
}

#[test]
fn parse_import_assigns_new_uuid() {
    let line = r#"{"kind":"rectangle"}"#;
    let r1 = parse_import_object_line(line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    let r2 = parse_import_object_line(line, Uuid::nil(), Uuid::nil())
        .unwrap()
        .unwrap();
    assert_ne!(r1.id, r2.id);
}
