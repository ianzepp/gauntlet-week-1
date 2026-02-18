#![allow(clippy::clone_on_copy, clippy::float_cmp)]

use serde_json::json;
use uuid::Uuid;

use super::*;

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

fn make_object_with_id(id: Uuid, kind: ObjectKind, z: i64) -> BoardObject {
    BoardObject {
        id,
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
// ObjectKind serde
// =============================================================

#[test]
fn kind_serde_roundtrip() {
    let json = serde_json::to_string(&ObjectKind::Diamond).unwrap();
    assert_eq!(json, "\"diamond\"");
    let back: ObjectKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ObjectKind::Diamond);
}

#[test]
fn kind_serde_all_variants() {
    let cases = [
        (ObjectKind::Rect, "\"rect\""),
        (ObjectKind::Ellipse, "\"ellipse\""),
        (ObjectKind::Diamond, "\"diamond\""),
        (ObjectKind::Star, "\"star\""),
        (ObjectKind::Line, "\"line\""),
        (ObjectKind::Arrow, "\"arrow\""),
    ];
    for (kind, expected) in cases {
        assert_eq!(serde_json::to_string(&kind).unwrap(), expected);
    }
}

#[test]
fn kind_deserialize_all_variants() {
    let cases = [
        ("\"rect\"", ObjectKind::Rect),
        ("\"ellipse\"", ObjectKind::Ellipse),
        ("\"diamond\"", ObjectKind::Diamond),
        ("\"star\"", ObjectKind::Star),
        ("\"line\"", ObjectKind::Line),
        ("\"arrow\"", ObjectKind::Arrow),
    ];
    for (input, expected) in cases {
        let kind: ObjectKind = serde_json::from_str(input).unwrap();
        assert_eq!(kind, expected);
    }
}

#[test]
fn kind_deserialize_invalid_rejects() {
    let result = serde_json::from_str::<ObjectKind>("\"hexagon\"");
    assert!(result.is_err());
}

#[test]
fn kind_clone_and_copy() {
    let a = ObjectKind::Star;
    let b = a;
    let c = a.clone();
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn kind_debug_format() {
    let s = format!("{:?}", ObjectKind::Rect);
    assert_eq!(s, "Rect");
}

// =============================================================
// BoardObject serde
// =============================================================

#[test]
fn board_object_serde_roundtrip() {
    let obj = BoardObject {
        id: Uuid::nil(),
        board_id: Uuid::nil(),
        kind: ObjectKind::Rect,
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
        rotation: 45.0,
        z_index: 3,
        props: json!({"fill": "#FF0000"}),
        created_by: Some(Uuid::nil()),
        version: 7,
    };
    let serialized = serde_json::to_string(&obj).unwrap();
    let back: BoardObject = serde_json::from_str(&serialized).unwrap();
    assert_eq!(back.id, obj.id);
    assert_eq!(back.kind, obj.kind);
    assert_eq!(back.x, obj.x);
    assert_eq!(back.y, obj.y);
    assert_eq!(back.width, obj.width);
    assert_eq!(back.height, obj.height);
    assert_eq!(back.rotation, obj.rotation);
    assert_eq!(back.z_index, obj.z_index);
    assert_eq!(back.props, obj.props);
    assert_eq!(back.created_by, obj.created_by);
    assert_eq!(back.version, obj.version);
}

#[test]
fn board_object_serde_created_by_null() {
    let obj = make_object(ObjectKind::Ellipse, 0);
    let serialized = serde_json::to_string(&obj).unwrap();
    let back: BoardObject = serde_json::from_str(&serialized).unwrap();
    assert_eq!(back.created_by, None);
}

#[test]
fn board_object_kind_serializes_lowercase() {
    let obj = make_object(ObjectKind::Diamond, 0);
    let serialized = serde_json::to_string(&obj).unwrap();
    assert!(serialized.contains("\"diamond\""));
    assert!(!serialized.contains("\"Diamond\""));
}

// =============================================================
// PartialBoardObject serde
// =============================================================

#[test]
fn partial_default_is_all_none() {
    let p = PartialBoardObject::default();
    assert!(p.x.is_none());
    assert!(p.y.is_none());
    assert!(p.width.is_none());
    assert!(p.height.is_none());
    assert!(p.rotation.is_none());
    assert!(p.z_index.is_none());
    assert!(p.props.is_none());
    assert!(p.version.is_none());
}

#[test]
fn partial_skip_serializing_none_fields() {
    let p = PartialBoardObject { x: Some(10.0), ..Default::default() };
    let serialized = serde_json::to_string(&p).unwrap();
    assert!(serialized.contains("\"x\""));
    assert!(!serialized.contains("\"y\""));
    assert!(!serialized.contains("\"width\""));
    assert!(!serialized.contains("\"height\""));
    assert!(!serialized.contains("\"rotation\""));
    assert!(!serialized.contains("\"z_index\""));
    assert!(!serialized.contains("\"props\""));
    assert!(!serialized.contains("\"version\""));
}

#[test]
fn partial_serde_roundtrip() {
    let p = PartialBoardObject {
        x: Some(1.0),
        y: Some(2.0),
        width: Some(3.0),
        height: Some(4.0),
        rotation: Some(5.0),
        z_index: Some(6),
        props: Some(json!({"fill": "#000"})),
        version: Some(7),
    };
    let serialized = serde_json::to_string(&p).unwrap();
    let back: PartialBoardObject = serde_json::from_str(&serialized).unwrap();
    assert_eq!(back.x, Some(1.0));
    assert_eq!(back.y, Some(2.0));
    assert_eq!(back.width, Some(3.0));
    assert_eq!(back.height, Some(4.0));
    assert_eq!(back.rotation, Some(5.0));
    assert_eq!(back.z_index, Some(6));
    assert_eq!(back.version, Some(7));
    assert_eq!(back.props.unwrap()["fill"], "#000");
}

// =============================================================
// DocStore: insert / get / remove
// =============================================================

#[test]
fn store_new_is_empty() {
    let store = DocStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
}

#[test]
fn store_default_is_empty() {
    let store = DocStore::default();
    assert!(store.is_empty());
}

#[test]
fn store_insert_and_get() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    assert_eq!(store.len(), 1);
    assert!(!store.is_empty());
    let retrieved = store.get(&id).unwrap();
    assert_eq!(retrieved.id, id);
}

#[test]
fn store_get_nonexistent_returns_none() {
    let store = DocStore::new();
    assert!(store.get(&Uuid::new_v4()).is_none());
}

#[test]
fn store_insert_overwrites_same_id() {
    let mut store = DocStore::new();
    let id = Uuid::new_v4();
    let obj1 = make_object_with_id(id, ObjectKind::Rect, 0);
    let mut obj2 = make_object_with_id(id, ObjectKind::Rect, 0);
    obj2.x = 999.0;
    store.insert(obj1);
    store.insert(obj2);
    assert_eq!(store.len(), 1);
    assert_eq!(store.get(&id).unwrap().x, 999.0);
}

#[test]
fn store_remove() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    let removed = store.remove(&id);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, id);
    assert!(store.is_empty());
}

#[test]
fn store_remove_nonexistent_returns_none() {
    let mut store = DocStore::new();
    assert!(store.remove(&Uuid::new_v4()).is_none());
}

#[test]
fn store_remove_does_not_affect_others() {
    let mut store = DocStore::new();
    let a = make_object(ObjectKind::Rect, 0);
    let b = make_object(ObjectKind::Ellipse, 0);
    let id_a = a.id;
    let id_b = b.id;
    store.insert(a);
    store.insert(b);
    store.remove(&id_a);
    assert_eq!(store.len(), 1);
    assert!(store.get(&id_b).is_some());
}

// =============================================================
// DocStore: apply_partial
// =============================================================

#[test]
fn apply_partial_x() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { x: Some(42.0), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().x, 42.0);
}

#[test]
fn apply_partial_y() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { y: Some(77.0), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().y, 77.0);
}

#[test]
fn apply_partial_width() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { width: Some(300.0), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().width, 300.0);
}

#[test]
fn apply_partial_height() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { height: Some(250.0), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().height, 250.0);
}

#[test]
fn apply_partial_rotation() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { rotation: Some(90.0), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().rotation, 90.0);
}

#[test]
fn apply_partial_z_index() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { z_index: Some(5), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().z_index, 5);
}

#[test]
fn apply_partial_version() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    store.apply_partial(&id, &PartialBoardObject { version: Some(42), ..Default::default() });
    assert_eq!(store.get(&id).unwrap().version, 42);
}

#[test]
fn apply_partial_multiple_fields() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    let partial = PartialBoardObject {
        x: Some(50.0),
        y: Some(60.0),
        width: Some(200.0),
        height: Some(150.0),
        ..Default::default()
    };
    assert!(store.apply_partial(&id, &partial));
    let updated = store.get(&id).unwrap();
    assert_eq!(updated.x, 50.0);
    assert_eq!(updated.y, 60.0);
    assert_eq!(updated.width, 200.0);
    assert_eq!(updated.height, 150.0);
    assert_eq!(updated.rotation, 0.0); // unchanged
    assert_eq!(updated.z_index, 0); // unchanged
}

#[test]
fn apply_partial_missing_id_returns_false() {
    let mut store = DocStore::new();
    let id = Uuid::new_v4();
    let partial = PartialBoardObject { x: Some(50.0), ..Default::default() };
    assert!(!store.apply_partial(&id, &partial));
}

#[test]
fn apply_partial_empty_is_noop() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);
    assert!(store.apply_partial(&id, &PartialBoardObject::default()));
    let obj = store.get(&id).unwrap();
    assert_eq!(obj.x, 0.0);
    assert_eq!(obj.y, 0.0);
    assert_eq!(obj.width, 100.0);
}

// =============================================================
// DocStore: apply_partial props merge
// =============================================================

#[test]
fn apply_partial_props_adds_new_key() {
    let mut store = DocStore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"fill": "#FF0000"});
    let id = obj.id;
    store.insert(obj);

    store.apply_partial(
        &id,
        &PartialBoardObject { props: Some(json!({"stroke": "#000000"})), ..Default::default() },
    );
    let updated = store.get(&id).unwrap();
    assert_eq!(updated.props["fill"], "#FF0000"); // preserved
    assert_eq!(updated.props["stroke"], "#000000"); // added
}

#[test]
fn apply_partial_props_updates_existing_key() {
    let mut store = DocStore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"fill": "#FF0000"});
    let id = obj.id;
    store.insert(obj);

    store.apply_partial(
        &id,
        &PartialBoardObject { props: Some(json!({"fill": "#00FF00"})), ..Default::default() },
    );
    assert_eq!(store.get(&id).unwrap().props["fill"], "#00FF00");
}

#[test]
fn apply_partial_props_null_removes_key() {
    let mut store = DocStore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"fill": "#FF0000", "stroke": "#000000"});
    let id = obj.id;
    store.insert(obj);

    store.apply_partial(
        &id,
        &PartialBoardObject { props: Some(json!({"stroke": null})), ..Default::default() },
    );
    let updated = store.get(&id).unwrap();
    assert_eq!(updated.props["fill"], "#FF0000");
    assert!(updated.props.get("stroke").is_none());
}

#[test]
fn apply_partial_props_multiple_ops_at_once() {
    let mut store = DocStore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!({"fill": "#FF0000", "stroke": "#000", "head": "old"});
    let id = obj.id;
    store.insert(obj);

    store.apply_partial(
        &id,
        &PartialBoardObject {
            props: Some(json!({
                "fill": "#00FF00",       // update
                "stroke": null,          // remove
                "text": "new"            // add
            })),
            ..Default::default()
        },
    );
    let p = &store.get(&id).unwrap().props;
    assert_eq!(p["fill"], "#00FF00");
    assert!(p.get("stroke").is_none());
    assert_eq!(p["head"], "old"); // untouched
    assert_eq!(p["text"], "new");
}

#[test]
fn apply_partial_props_initializes_non_object_existing_props() {
    let mut store = DocStore::new();
    let mut obj = make_object(ObjectKind::Rect, 0);
    obj.props = json!(null);
    let id = obj.id;
    store.insert(obj);

    assert!(store.apply_partial(
        &id,
        &PartialBoardObject { props: Some(json!({"fill": "#00FF00"})), ..Default::default() },
    ));
    assert_eq!(store.get(&id).unwrap().props["fill"], "#00FF00");
}

#[test]
fn apply_partial_props_non_object_patch_returns_false() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 0);
    let id = obj.id;
    store.insert(obj);

    assert!(!store.apply_partial(&id, &PartialBoardObject { props: Some(json!(42)), ..Default::default() },));
}

// =============================================================
// DocStore: load_snapshot
// =============================================================

#[test]
fn load_snapshot_replaces_existing() {
    let mut store = DocStore::new();
    let existing = make_object(ObjectKind::Rect, 0);
    let existing_id = existing.id;
    store.insert(existing);

    let new1 = make_object(ObjectKind::Ellipse, 0);
    let new2 = make_object(ObjectKind::Star, 1);
    let new1_id = new1.id;
    store.load_snapshot(vec![new1, new2]);

    assert_eq!(store.len(), 2);
    assert!(store.get(&existing_id).is_none()); // old one gone
    assert!(store.get(&new1_id).is_some());
}

#[test]
fn load_snapshot_empty_clears_store() {
    let mut store = DocStore::new();
    store.insert(make_object(ObjectKind::Rect, 0));
    store.load_snapshot(vec![]);
    assert!(store.is_empty());
}

// =============================================================
// DocStore: sorted_objects
// =============================================================

#[test]
fn sorted_objects_empty() {
    let store = DocStore::new();
    assert!(store.sorted_objects().is_empty());
}

#[test]
fn sorted_objects_by_z_index() {
    let mut store = DocStore::new();
    store.insert(make_object(ObjectKind::Rect, 3));
    store.insert(make_object(ObjectKind::Ellipse, 1));
    store.insert(make_object(ObjectKind::Star, 2));

    let sorted = store.sorted_objects();
    assert_eq!(sorted[0].z_index, 1);
    assert_eq!(sorted[1].z_index, 2);
    assert_eq!(sorted[2].z_index, 3);
}

#[test]
fn sorted_objects_tiebreak_by_id() {
    let mut store = DocStore::new();
    // Use deterministic IDs to verify sort order
    let id_low = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let id_high = Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap();

    // Insert high first to ensure sort isn't just insertion order
    store.insert(make_object_with_id(id_high, ObjectKind::Rect, 1));
    store.insert(make_object_with_id(id_low, ObjectKind::Ellipse, 1));

    let sorted = store.sorted_objects();
    assert_eq!(sorted[0].id, id_low);
    assert_eq!(sorted[1].id, id_high);
}

#[test]
fn sorted_objects_single_element() {
    let mut store = DocStore::new();
    let obj = make_object(ObjectKind::Rect, 5);
    let id = obj.id;
    store.insert(obj);
    let sorted = store.sorted_objects();
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0].id, id);
}

#[test]
fn sorted_objects_negative_z_index() {
    let mut store = DocStore::new();
    store.insert(make_object(ObjectKind::Rect, 0));
    store.insert(make_object(ObjectKind::Ellipse, -1));

    let sorted = store.sorted_objects();
    assert_eq!(sorted[0].z_index, -1);
    assert_eq!(sorted[1].z_index, 0);
}

// =============================================================
// Props
// =============================================================

#[test]
fn props_defaults_on_empty_object() {
    let value = json!({});
    let p = Props::new(&value);
    assert_eq!(p.fill(), "#D94B4B");
    assert_eq!(p.stroke(), "#1F1A17");
    assert_eq!(p.stroke_width(), 1.0);
    assert_eq!(p.head(), "");
    assert_eq!(p.text(), "");
    assert_eq!(p.foot(), "");
}

#[test]
fn props_reads_all_values() {
    let value = json!({
        "fill": "#AABBCC",
        "stroke": "#112233",
        "stroke_width": 3.0,
        "head": "Title",
        "text": "Body",
        "foot": "Footer"
    });
    let p = Props::new(&value);
    assert_eq!(p.fill(), "#AABBCC");
    assert_eq!(p.stroke(), "#112233");
    assert_eq!(p.stroke_width(), 3.0);
    assert_eq!(p.head(), "Title");
    assert_eq!(p.text(), "Body");
    assert_eq!(p.foot(), "Footer");
}

#[test]
fn props_partial_fields_use_defaults() {
    let value = json!({"fill": "#123456"});
    let p = Props::new(&value);
    assert_eq!(p.fill(), "#123456");
    assert_eq!(p.stroke(), "#1F1A17"); // default
    assert_eq!(p.stroke_width(), 1.0); // default
    assert_eq!(p.head(), ""); // default
}

#[test]
fn props_stroke_width_integer_coerces() {
    let value = json!({"stroke_width": 2});
    let p = Props::new(&value);
    assert_eq!(p.stroke_width(), 2.0);
}

#[test]
fn props_wrong_type_uses_default() {
    let value = json!({"fill": 42, "stroke_width": "thick"});
    let p = Props::new(&value);
    assert_eq!(p.fill(), "#D94B4B"); // 42 is not a string
    assert_eq!(p.stroke_width(), 1.0); // "thick" is not a number
}

#[test]
fn props_text_with_newlines() {
    let value = json!({"head": "line1\nline2", "text": "a\nb\nc"});
    let p = Props::new(&value);
    assert_eq!(p.head(), "line1\nline2");
    assert_eq!(p.text(), "a\nb\nc");
}
