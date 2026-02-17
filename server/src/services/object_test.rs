use super::*;
use crate::state::test_helpers;

#[tokio::test]
async fn create_object_succeeds() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "sticky_note",
        10.0,
        20.0,
        serde_json::json!({"text": "hi"}),
        None,
    )
    .await
    .unwrap();
    assert_eq!(obj.kind, "sticky_note");
    assert!((obj.x - 10.0).abs() < f64::EPSILON);
    assert!((obj.y - 20.0).abs() < f64::EPSILON);
    assert_eq!(obj.version, 1);

    // Verify in-memory state
    let boards = state.boards.read().await;
    let board = boards.get(&board_id).unwrap();
    assert!(board.objects.contains_key(&obj.id));
    assert!(board.dirty.contains(&obj.id));
}

#[tokio::test]
async fn create_object_board_not_loaded() {
    let state = test_helpers::test_app_state();
    let fake_id = Uuid::new_v4();
    let result = create_object(&state, fake_id, "sticky_note", 0.0, 0.0, serde_json::json!({}), None).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ObjectError::BoardNotLoaded(_)));
}

#[tokio::test]
async fn update_object_succeeds() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(&state, board_id, "rectangle", 0.0, 0.0, serde_json::json!({}), None)
        .await
        .unwrap();

    let mut data = Data::new();
    data.insert("x".into(), serde_json::json!(50.0));
    data.insert("y".into(), serde_json::json!(75.0));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert!((updated.x - 50.0).abs() < f64::EPSILON);
    assert!((updated.y - 75.0).abs() < f64::EPSILON);
    assert_eq!(updated.version, 2);
}

#[tokio::test]
async fn update_object_lww_rejects_stale() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(&state, board_id, "ellipse", 0.0, 0.0, serde_json::json!({}), None)
        .await
        .unwrap();
    assert_eq!(obj.version, 1);

    // Update with version 1 succeeds (incoming >= current)
    let mut data = Data::new();
    data.insert("x".into(), serde_json::json!(10.0));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.version, 2);

    // Update with version 0 fails (incoming < current)
    let result = update_object(&state, board_id, obj.id, &data, 0).await;
    assert!(matches!(
        result.unwrap_err(),
        ObjectError::StaleUpdate { incoming: 0, current: 2 }
    ));
}

#[tokio::test]
async fn update_object_not_found() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let data = Data::new();
    let result = update_object(&state, board_id, Uuid::new_v4(), &data, 0).await;
    assert!(matches!(result.unwrap_err(), ObjectError::NotFound(_)));
}

#[tokio::test]
async fn update_object_partial_fields() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(&state, board_id, "text", 10.0, 20.0, serde_json::json!({}), None)
        .await
        .unwrap();

    // Only update x, leave y unchanged
    let mut data = Data::new();
    data.insert("x".into(), serde_json::json!(99.0));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert!((updated.x - 99.0).abs() < f64::EPSILON);
    assert!((updated.y - 20.0).abs() < f64::EPSILON); // unchanged
}

#[tokio::test]
async fn update_object_props() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "sticky_note",
        0.0,
        0.0,
        serde_json::json!({"text": "old"}),
        None,
    )
    .await
    .unwrap();

    let mut data = Data::new();
    data.insert("props".into(), serde_json::json!({"text": "new", "color": "#FF0000"}));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.props.get("text").unwrap().as_str().unwrap(), "new");
    assert_eq!(updated.props.get("color").unwrap().as_str().unwrap(), "#FF0000");
}

#[tokio::test]
async fn create_object_marks_dirty() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(&state, board_id, "rectangle", 0.0, 0.0, serde_json::json!({}), None)
        .await
        .unwrap();

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).unwrap();
    assert!(board.dirty.contains(&obj.id));
}

#[tokio::test]
#[ignore = "delete_object hits Postgres via sqlx::query"]
async fn delete_object_removes_from_memory() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(&state, board_id, "rectangle", 0.0, 0.0, serde_json::json!({}), None)
        .await
        .unwrap();
    let _ = delete_object(&state, board_id, obj.id).await;
}
