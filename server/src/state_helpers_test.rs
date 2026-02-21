use super::*;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

/// Create a test `AppState` with a dummy `PgPool` (`connect_lazy`, no live DB).
#[must_use]
pub fn test_app_state() -> AppState {
    let pool = PgPoolOptions::new()
        // Fail fast if a test accidentally performs real DB I/O.
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("postgres://test:test@127.0.0.1:1/test_gauntlet_week_1")
        .expect("connect_lazy should not fail");
    AppState::new(pool, None, None)
}

/// Create a test `AppState` with a mock LLM.
#[must_use]
pub fn test_app_state_with_llm(llm: Arc<dyn LlmChat>) -> AppState {
    let pool = PgPoolOptions::new()
        // Fail fast if a test accidentally performs real DB I/O.
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("postgres://test:test@127.0.0.1:1/test_gauntlet_week_1")
        .expect("connect_lazy should not fail");
    AppState::new(pool, Some(llm), None)
}

/// Seed an empty board into the app state and return its ID.
pub async fn seed_board(state: &AppState) -> Uuid {
    let board_id = Uuid::new_v4();
    let mut boards = state.boards.write().await;
    boards.insert(board_id, BoardState::new());
    board_id
}

/// Seed a board with pre-populated objects and return the board ID.
pub async fn seed_board_with_objects(state: &AppState, objects: Vec<BoardObject>) -> Uuid {
    let board_id = Uuid::new_v4();
    let mut board_state = BoardState::new();
    for mut obj in objects {
        obj.board_id = board_id;
        board_state.objects.insert(obj.id, obj);
    }
    let mut boards = state.boards.write().await;
    boards.insert(board_id, board_state);
    board_id
}

/// Create a dummy `BoardObject` for testing.
#[must_use]
pub fn dummy_object() -> BoardObject {
    BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind: "sticky_note".into(),
        x: 100.0,
        y: 200.0,
        width: None,
        height: None,
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({"text": "test", "color": "#FFEB3B"}),
        created_by: None,
        version: 1,
        group_id: None,
    }
}
