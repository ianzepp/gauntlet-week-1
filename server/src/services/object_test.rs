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
        None,
        None,
        0.0,
        serde_json::json!({"text": "hi"}),
        None,
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
    let result = create_object(
        &state,
        fake_id,
        "sticky_note",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ObjectError::BoardNotLoaded(_)));
}

#[tokio::test]
async fn update_object_succeeds() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
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
    let obj = create_object(
        &state,
        board_id,
        "ellipse",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
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
    let obj = create_object(
        &state,
        board_id,
        "text",
        10.0,
        20.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
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
        None,
        None,
        0.0,
        serde_json::json!({"text": "old"}),
        None,
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
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).unwrap();
    assert!(board.dirty.contains(&obj.id));
}

#[tokio::test]
async fn update_object_z_index_from_integer() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();

    let mut data = Data::new();
    data.insert("z_index".into(), serde_json::json!(7));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.z_index, 7);
}

#[tokio::test]
async fn update_object_z_index_from_float_whole_number() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();

    let mut data = Data::new();
    data.insert("z_index".into(), serde_json::json!(5.0));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.z_index, 5);
}

#[tokio::test]
async fn update_object_z_index_from_float_fractional_ignored() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();
    let original_z = obj.z_index;

    let mut data = Data::new();
    data.insert("z_index".into(), serde_json::json!(5.5));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.z_index, original_z);
}

#[tokio::test]
async fn update_object_rotation_field() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();

    let mut data = Data::new();
    data.insert("rotation".into(), serde_json::json!(45.0));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert!((updated.rotation - 45.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn update_object_width_height_fields() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();
    assert!(obj.width.is_none());
    assert!(obj.height.is_none());

    let mut data = Data::new();
    data.insert("width".into(), serde_json::json!(200.0));
    data.insert("height".into(), serde_json::json!(150.0));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.width, Some(200.0));
    assert_eq!(updated.height, Some(150.0));
}

#[tokio::test]
async fn update_object_group_id_valid_uuid() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();

    let group = Uuid::new_v4();
    let mut data = Data::new();
    data.insert("group_id".into(), serde_json::json!(group.to_string()));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.group_id, Some(group));
}

#[tokio::test]
async fn update_object_group_id_null_clears() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let group = Uuid::new_v4();
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        Some(group),
    )
    .await
    .unwrap();
    assert_eq!(obj.group_id, Some(group));

    let mut data = Data::new();
    data.insert("group_id".into(), serde_json::Value::Null);
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.group_id, None);
}

#[tokio::test]
async fn update_object_props_replaces_entirely() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "sticky_note",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({"text": "old", "color": "#FF0000"}),
        None,
        None,
    )
    .await
    .unwrap();

    let mut data = Data::new();
    data.insert("props".into(), serde_json::json!({"text": "new"}));
    let updated = update_object(&state, board_id, obj.id, &data, 1)
        .await
        .unwrap();
    assert_eq!(updated.props.get("text").unwrap().as_str().unwrap(), "new");
    assert!(
        updated.props.get("color").is_none(),
        "old key 'color' should be gone after full props replace"
    );
}

#[tokio::test]
async fn update_object_sequential_version_increments() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(obj.version, 1);

    let mut data1 = Data::new();
    data1.insert("x".into(), serde_json::json!(10.0));
    let v2 = update_object(&state, board_id, obj.id, &data1, 1)
        .await
        .unwrap();
    assert_eq!(v2.version, 2);

    let mut data2 = Data::new();
    data2.insert("x".into(), serde_json::json!(20.0));
    let v3 = update_object(&state, board_id, obj.id, &data2, 2)
        .await
        .unwrap();
    assert_eq!(v3.version, 3);
}

#[tokio::test]
#[ignore = "delete_object hits Postgres via sqlx::query"]
async fn delete_object_removes_from_memory() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let obj = create_object(
        &state,
        board_id,
        "rectangle",
        0.0,
        0.0,
        None,
        None,
        0.0,
        serde_json::json!({}),
        None,
        None,
    )
    .await
    .unwrap();
    let _ = delete_object(&state, board_id, obj.id).await;
}
