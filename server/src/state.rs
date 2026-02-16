//! Shared application state.
//!
//! DESIGN
//! ======
//! `AppState` is injected into Axum handlers via the `State` extractor.
//! It holds the database pool and a map of live board states. Each board
//! has its own in-memory object store, connected clients, and dirty set
//! for debounced persistence.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::frame::Frame;
use crate::llm::LlmChat;
use crate::rate_limit::RateLimiter;

// =============================================================================
// BOARD OBJECT
// =============================================================================

/// In-memory representation of a board object. Mirrors the `board_objects` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardObject {
    pub id: Uuid,
    pub board_id: Uuid,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: f64,
    pub z_index: i32,
    pub props: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub version: i32,
}

// =============================================================================
// BOARD STATE
// =============================================================================

/// Per-board live state. Kept in memory for real-time performance.
/// Flushed to Postgres by the persistence task.
pub struct BoardState {
    /// Current objects keyed by object ID.
    pub objects: HashMap<Uuid, BoardObject>,
    /// Connected clients: `client_id` -> sender for outgoing frames.
    pub clients: HashMap<Uuid, mpsc::Sender<Frame>>,
    /// Object IDs modified since last flush.
    pub dirty: HashSet<Uuid>,
}

impl BoardState {
    #[must_use]
    pub fn new() -> Self {
        Self { objects: HashMap::new(), clients: HashMap::new(), dirty: HashSet::new() }
    }
}

impl Default for BoardState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// APP STATE
// =============================================================================

/// Shared application state, injected into Axum handlers via State extractor.
/// Clone is required by Axum — all inner fields are Arc-wrapped or Clone.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub boards: Arc<RwLock<HashMap<Uuid, BoardState>>>,
    /// Optional LLM client. `None` if LLM env vars are not configured.
    pub llm: Option<Arc<dyn LlmChat>>,
    /// In-memory rate limiter for AI requests.
    pub rate_limiter: RateLimiter,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool, llm: Option<Arc<dyn LlmChat>>) -> Self {
        Self { pool, boards: Arc::new(RwLock::new(HashMap::new())), llm, rate_limiter: RateLimiter::new() }
    }
}

// =============================================================================
// TEST HELPERS
// =============================================================================

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    /// Create a test `AppState` with a dummy `PgPool` (connect_lazy, no live DB).
    #[must_use]
    pub fn test_app_state() -> AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost:5432/test_collaboard")
            .expect("connect_lazy should not fail");
        AppState::new(pool, None)
    }

    /// Create a test `AppState` with a mock LLM.
    #[must_use]
    pub fn test_app_state_with_llm(llm: Arc<dyn LlmChat>) -> AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost:5432/test_collaboard")
            .expect("connect_lazy should not fail");
        AppState::new(pool, Some(llm))
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
        }
    }
}

#[cfg(test)]
mod tests {
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
}
