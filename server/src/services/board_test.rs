use super::*;
use crate::frame::{Data, Frame};
#[cfg(feature = "live-db-tests")]
use crate::state::AppState;
use crate::state::{BoardObject, BoardState, test_helpers};
#[cfg(feature = "live-db-tests")]
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

async fn assert_channel_has_frame(rx: &mut mpsc::Receiver<Frame>) -> Frame {
    timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("frame receive timed out")
        .expect("channel closed")
}

async fn assert_channel_empty(rx: &mut mpsc::Receiver<Frame>) {
    assert!(
        timeout(Duration::from_millis(80), rx.recv()).await.is_err(),
        "expected channel to remain empty"
    );
}

#[tokio::test]
async fn broadcast_sends_to_all_except_excluded_client() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;

    let client_a = Uuid::new_v4();
    let client_b = Uuid::new_v4();
    let client_c = Uuid::new_v4();

    let (tx_a, mut rx_a) = mpsc::channel(8);
    let (tx_b, mut rx_b) = mpsc::channel(8);
    let (tx_c, mut rx_c) = mpsc::channel(8);

    {
        let mut boards = state.boards.write().await;
        let board = boards.get_mut(&board_id).expect("board should exist");
        board.clients.insert(client_a, tx_a);
        board.clients.insert(client_b, tx_b);
        board.clients.insert(client_c, tx_c);
    }

    let frame = Frame::request("object:update", Data::new()).with_board_id(board_id);
    broadcast(&state, board_id, &frame, Some(client_b)).await;

    let recv_a = assert_channel_has_frame(&mut rx_a).await;
    let recv_c = assert_channel_has_frame(&mut rx_c).await;
    assert_eq!(recv_a.syscall, "object:update");
    assert_eq!(recv_c.syscall, "object:update");
    assert_channel_empty(&mut rx_b).await;
}

#[tokio::test]
async fn part_board_removes_client_but_keeps_board_with_other_clients() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;

    let client_a = Uuid::new_v4();
    let client_b = Uuid::new_v4();
    let (tx_a, _rx_a) = mpsc::channel(8);
    let (tx_b, _rx_b) = mpsc::channel(8);

    {
        let mut boards = state.boards.write().await;
        let board = boards.get_mut(&board_id).expect("board should exist");
        board.clients.insert(client_a, tx_a);
        board.clients.insert(client_b, tx_b);
    }

    part_board(&state, board_id, client_a).await;

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should remain loaded");
    assert!(!board.clients.contains_key(&client_a));
    assert!(board.clients.contains_key(&client_b));
}

#[tokio::test]
async fn part_board_evicts_clean_board_when_last_client_leaves() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;

    let client = Uuid::new_v4();
    let (tx, _rx) = mpsc::channel(8);
    {
        let mut boards = state.boards.write().await;
        let board = boards.get_mut(&board_id).expect("board should exist");
        board.clients.insert(client, tx);
    }

    part_board(&state, board_id, client).await;

    let boards = state.boards.read().await;
    assert!(
        !boards.contains_key(&board_id),
        "board should be evicted after last clean client leaves"
    );
}

#[tokio::test]
async fn part_board_evicts_dirty_board_even_if_flush_fails() {
    let state = test_helpers::test_app_state();
    let board_id = Uuid::new_v4();
    let client = Uuid::new_v4();

    let object = BoardObject {
        id: Uuid::new_v4(),
        board_id,
        kind: "sticky_note".into(),
        x: 10.0,
        y: 20.0,
        width: None,
        height: None,
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({"text": "dirty"}),
        created_by: None,
        version: 1,
    };

    let (tx, _rx) = mpsc::channel(8);
    let mut board_state = BoardState::new();
    let object_id = object.id;
    board_state.objects.insert(object_id, object);
    board_state.dirty.insert(object_id);
    board_state.clients.insert(client, tx);

    {
        let mut boards = state.boards.write().await;
        boards.insert(board_id, board_state);
    }

    // With connect_lazy test state, DB flush may fail. The board should still be evicted.
    part_board(&state, board_id, client).await;

    let boards = state.boards.read().await;
    assert!(
        !boards.contains_key(&board_id),
        "board should be evicted after last client leaves, even after flush attempt"
    );
}

#[cfg(feature = "live-db-tests")]
async fn integration_pool() -> sqlx::PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test:test@localhost:5432/test_gauntlet_week_1".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("requires reachable Postgres; set TEST_DATABASE_URL");

    sqlx::migrate!("src/db/migrations")
        .run(&pool)
        .await
        .expect("migrations should run");

    sqlx::query("TRUNCATE TABLE board_objects, boards RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("test cleanup should succeed");

    pool
}

#[cfg(feature = "live-db-tests")]
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL/live Postgres"]
async fn board_crud_round_trip_with_list_and_delete() {
    let pool = integration_pool().await;
    let owner_id = Uuid::new_v4();

    let row = create_board(&pool, "Integration Board", owner_id)
        .await
        .expect("create_board should succeed");
    let listed = list_boards(&pool, owner_id)
        .await
        .expect("list_boards should succeed");
    assert!(
        listed
            .iter()
            .any(|b| b.id == row.id && b.name == "Integration Board")
    );

    delete_board(&pool, row.id, owner_id)
        .await
        .expect("delete_board should succeed");
    let listed_after = list_boards(&pool, owner_id)
        .await
        .expect("list_boards should succeed after delete");
    assert!(!listed_after.iter().any(|b| b.id == row.id));

    let missing = delete_board(&pool, Uuid::new_v4(), owner_id).await;
    assert!(matches!(missing, Err(BoardError::NotFound(_))));
}

#[cfg(feature = "live-db-tests")]
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL/live Postgres"]
async fn join_board_hydrates_objects_from_database() {
    let pool = integration_pool().await;
    let owner_id = Uuid::new_v4();
    let board = create_board(&pool, "Hydration Board", owner_id)
        .await
        .expect("create_board should succeed");

    let obj = BoardObject {
        id: Uuid::new_v4(),
        board_id: board.id,
        kind: "sticky_note".into(),
        x: 42.0,
        y: 84.0,
        width: None,
        height: None,
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({"text": "seeded"}),
        created_by: None,
        version: 1,
    };
    flush_objects(&pool, std::slice::from_ref(&obj))
        .await
        .expect("flush_objects should seed row");

    let state = AppState::new(pool, None, None);
    let client_id = Uuid::new_v4();
    let (tx, _rx) = mpsc::channel(8);

    let hydrated = join_board(&state, board.id, owner_id, client_id, tx)
        .await
        .expect("join_board should hydrate objects");

    assert_eq!(hydrated.len(), 1);
    assert_eq!(hydrated[0].id, obj.id);
    assert_eq!(hydrated[0].props.get("text").and_then(|v| v.as_str()), Some("seeded"));

    let boards = state.boards.read().await;
    let loaded = boards.get(&board.id).expect("board should be loaded");
    assert!(loaded.clients.contains_key(&client_id));
    assert!(loaded.objects.contains_key(&obj.id));
}

#[cfg(feature = "live-db-tests")]
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL/live Postgres"]
async fn part_board_flushes_dirty_object_to_database_on_last_client() {
    let pool = integration_pool().await;
    let board = create_board(&pool, "Flush Board", Uuid::new_v4())
        .await
        .expect("create_board should succeed");

    let state = AppState::new(pool.clone(), None, None);
    let client_id = Uuid::new_v4();

    let obj = BoardObject {
        id: Uuid::new_v4(),
        board_id: board.id,
        kind: "sticky_note".into(),
        x: 300.0,
        y: 150.0,
        width: Some(240.0),
        height: Some(120.0),
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({"text": "flush me"}),
        created_by: None,
        version: 2,
    };

    let (tx, _rx) = mpsc::channel(8);
    let mut board_state = BoardState::new();
    board_state.clients.insert(client_id, tx);
    board_state.objects.insert(obj.id, obj.clone());
    board_state.dirty.insert(obj.id);

    {
        let mut boards = state.boards.write().await;
        boards.insert(board.id, board_state);
    }

    part_board(&state, board.id, client_id).await;

    let boards = state.boards.read().await;
    assert!(!boards.contains_key(&board.id));

    let persisted = sqlx::query_as::<_, (Uuid, f64, f64, Option<f64>, Option<f64>, i32)>(
        "SELECT id, x, y, width, height, version FROM board_objects WHERE id = $1",
    )
    .bind(obj.id)
    .fetch_optional(&pool)
    .await
    .expect("select should work")
    .expect("dirty object should be flushed to DB");

    assert_eq!(persisted.0, obj.id);
    assert!((persisted.1 - 300.0).abs() < f64::EPSILON);
    assert!((persisted.2 - 150.0).abs() < f64::EPSILON);
    assert_eq!(persisted.3, Some(240.0));
    assert_eq!(persisted.4, Some(120.0));
    assert_eq!(persisted.5, 2);
}
