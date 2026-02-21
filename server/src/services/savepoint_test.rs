use super::*;
use uuid::Uuid;

#[test]
fn savepoint_row_to_json_includes_all_fields() {
    let id = Uuid::new_v4();
    let board_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let row = SavepointRow {
        id,
        board_id,
        seq: 42,
        ts: 1700000000000,
        created_by: Some(user_id),
        is_auto: true,
        reason: "test reason".to_owned(),
        label: Some("my label".to_owned()),
        snapshot: serde_json::json!([{"kind": "rect"}]),
    };
    let json = savepoint_row_to_json(row);
    assert_eq!(json["id"], serde_json::json!(id));
    assert_eq!(json["board_id"], serde_json::json!(board_id));
    assert_eq!(json["seq"], 42);
    assert_eq!(json["ts"], 1_700_000_000_000_i64);
    assert_eq!(json["created_by"], serde_json::json!(user_id));
    assert_eq!(json["is_auto"], true);
    assert_eq!(json["reason"], "test reason");
    assert_eq!(json["label"], "my label");
    assert_eq!(json["snapshot"], serde_json::json!([{"kind": "rect"}]));
}

#[test]
fn savepoint_row_to_json_handles_null_optional_fields() {
    let row = SavepointRow {
        id: Uuid::nil(),
        board_id: Uuid::nil(),
        seq: 0,
        ts: 0,
        created_by: None,
        is_auto: false,
        reason: String::new(),
        label: None,
        snapshot: serde_json::json!([]),
    };
    let json = savepoint_row_to_json(row);
    assert!(json["created_by"].is_null());
    assert!(json["label"].is_null());
}

#[test]
fn savepoint_rows_to_json_maps_all_rows() {
    let rows = vec![
        SavepointRow {
            id: Uuid::new_v4(),
            board_id: Uuid::nil(),
            seq: 1,
            ts: 100,
            created_by: None,
            is_auto: false,
            reason: "a".to_owned(),
            label: None,
            snapshot: serde_json::json!([]),
        },
        SavepointRow {
            id: Uuid::new_v4(),
            board_id: Uuid::nil(),
            seq: 2,
            ts: 200,
            created_by: None,
            is_auto: true,
            reason: "b".to_owned(),
            label: None,
            snapshot: serde_json::json!([]),
        },
    ];
    let result = savepoint_rows_to_json(rows);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0]["seq"], 1);
    assert_eq!(result[1]["seq"], 2);
}
