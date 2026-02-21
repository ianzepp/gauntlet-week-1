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

// =============================================================================
// BoardState: insert / remove objects
// =============================================================================

#[test]
fn board_state_insert_and_retrieve_object() {
    let mut bs = BoardState::new();
    let obj = test_helpers::dummy_object();
    let id = obj.id;
    bs.objects.insert(id, obj);
    assert_eq!(bs.objects.len(), 1);
    assert_eq!(bs.objects.get(&id).unwrap().kind, "sticky_note");
}

#[test]
fn board_state_remove_object() {
    let mut bs = BoardState::new();
    let obj = test_helpers::dummy_object();
    let id = obj.id;
    bs.objects.insert(id, obj);
    assert!(bs.objects.remove(&id).is_some());
    assert!(bs.objects.is_empty());
}

// =============================================================================
// Dirty tracking
// =============================================================================

#[test]
fn board_state_dirty_tracking() {
    let mut bs = BoardState::new();
    let obj = test_helpers::dummy_object();
    let id = obj.id;
    bs.objects.insert(id, obj);
    bs.dirty.insert(id);

    assert!(bs.dirty.contains(&id));
    assert_eq!(bs.dirty.len(), 1);

    bs.dirty.clear();
    assert!(bs.dirty.is_empty());
}

#[test]
fn board_state_dirty_only_tracks_inserted_ids() {
    let mut bs = BoardState::new();
    let phantom_id = Uuid::new_v4();
    bs.dirty.insert(phantom_id);
    // dirty can contain IDs not in objects (this is a property of the design)
    assert!(bs.dirty.contains(&phantom_id));
    assert!(bs.objects.get(&phantom_id).is_none());
}

// =============================================================================
// Add / remove clients
// =============================================================================

#[test]
fn board_state_add_and_remove_client() {
    let mut bs = BoardState::new();
    let client_id = Uuid::new_v4();
    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    bs.clients.insert(client_id, tx);
    assert_eq!(bs.clients.len(), 1);

    bs.clients.remove(&client_id);
    assert!(bs.clients.is_empty());
}

// =============================================================================
// BoardObject: optional fields None
// =============================================================================

#[test]
fn board_object_optional_fields_none() {
    let obj = BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind: "shape".into(),
        x: 0.0,
        y: 0.0,
        width: None,
        height: None,
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({}),
        created_by: None,
        version: 1,
        group_id: None,
    };
    let json = serde_json::to_string(&obj).unwrap();
    let restored: BoardObject = serde_json::from_str(&json).unwrap();
    assert!(restored.width.is_none());
    assert!(restored.height.is_none());
    assert!(restored.created_by.is_none());
}

#[test]
fn board_object_optional_fields_some() {
    let user_id = Uuid::new_v4();
    let obj = BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind: "sticky_note".into(),
        x: 10.0,
        y: 20.0,
        width: Some(200.0),
        height: Some(150.0),
        rotation: 45.0,
        z_index: 3,
        props: serde_json::json!({"text": "hello"}),
        created_by: Some(user_id),
        version: 5,
        group_id: None,
    };
    let json = serde_json::to_string(&obj).unwrap();
    let restored: BoardObject = serde_json::from_str(&json).unwrap();
    assert!((restored.width.unwrap() - 200.0).abs() < f64::EPSILON);
    assert!((restored.height.unwrap() - 150.0).abs() < f64::EPSILON);
    assert_eq!(restored.created_by, Some(user_id));
    assert_eq!(restored.version, 5);
    assert_eq!(restored.z_index, 3);
    assert!((restored.rotation - 45.0).abs() < f64::EPSILON);
}

// =============================================================================
// created_by null in JSON
// =============================================================================

#[test]
fn board_object_created_by_null_serde() {
    let json = serde_json::json!({
        "id": Uuid::new_v4(),
        "board_id": Uuid::new_v4(),
        "kind": "sticky_note",
        "x": 0.0,
        "y": 0.0,
        "width": null,
        "height": null,
        "rotation": 0.0,
        "z_index": 0,
        "props": {},
        "created_by": null,
        "version": 1
    });
    let obj: BoardObject = serde_json::from_value(json).unwrap();
    assert!(obj.created_by.is_none());
}

// =============================================================================
// Version preserved through serde
// =============================================================================

#[test]
fn board_object_version_preserved() {
    let mut obj = test_helpers::dummy_object();
    obj.version = 42;
    let json = serde_json::to_string(&obj).unwrap();
    let restored: BoardObject = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.version, 42);
}
