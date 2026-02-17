use super::*;

#[test]
fn board_state_new_is_empty() {
    let bs = BoardState::new();
    assert!(bs.objects.is_empty());
    assert!(bs.clients.is_empty());
    assert!(bs.dirty.is_empty());
}

#[test]
fn board_object_serde_round_trip() {
    let obj = test_helpers::dummy_object();
    let json = serde_json::to_string(&obj).unwrap();
    let restored: BoardObject = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, obj.id);
    assert_eq!(restored.kind, "sticky_note");
    assert!((restored.x - 100.0).abs() < f64::EPSILON);
    assert!((restored.y - 200.0).abs() < f64::EPSILON);
    assert_eq!(restored.version, 1);
}

#[test]
fn board_state_default_equals_new() {
    let a = BoardState::new();
    let b = BoardState::default();
    assert_eq!(a.objects.len(), b.objects.len());
    assert_eq!(a.clients.len(), b.clients.len());
    assert_eq!(a.dirty.len(), b.dirty.len());
}
