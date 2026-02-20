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
use crate::llm::types::Message;
use crate::rate_limit::RateLimiter;
use crate::services::auth::GitHubConfig;

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
#[derive(Debug, Clone)]
pub struct ConnectedClient {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_color: String,
    pub can_edit: bool,
    pub can_admin: bool,
}

pub struct BoardState {
    /// Current objects keyed by object ID.
    pub objects: HashMap<Uuid, BoardObject>,
    /// Connected clients: `client_id` -> sender for outgoing frames.
    pub clients: HashMap<Uuid, mpsc::Sender<Frame>>,
    /// Connected user metadata for each client.
    pub users: HashMap<Uuid, ConnectedClient>,
    /// Object IDs modified since last flush.
    pub dirty: HashSet<Uuid>,
}

impl BoardState {
    #[must_use]
    pub fn new() -> Self {
        Self { objects: HashMap::new(), clients: HashMap::new(), users: HashMap::new(), dirty: HashSet::new() }
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
    pub ws_clients: Arc<RwLock<HashMap<Uuid, mpsc::Sender<Frame>>>>,
    /// Optional bounded queue sender for async frame persistence.
    /// `None` in tests or when frame persistence is disabled.
    pub frame_persist_tx: Option<mpsc::Sender<Frame>>,
    /// Optional LLM client. `None` if LLM env vars are not configured.
    pub llm: Option<Arc<dyn LlmChat>>,
    /// In-memory rate limiter for AI requests.
    pub rate_limiter: RateLimiter,
    /// AI conversation memory scoped to active websocket session and board.
    pub ai_session_messages: Arc<RwLock<HashMap<(Uuid, Uuid), Vec<Message>>>>,
    /// Optional GitHub OAuth config. `None` disables OAuth endpoints.
    pub github: Option<GitHubConfig>,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool, llm: Option<Arc<dyn LlmChat>>, github: Option<GitHubConfig>) -> Self {
        Self {
            pool,
            boards: Arc::new(RwLock::new(HashMap::new())),
            ws_clients: Arc::new(RwLock::new(HashMap::new())),
            frame_persist_tx: None,
            llm,
            rate_limiter: RateLimiter::new(),
            ai_session_messages: Arc::new(RwLock::new(HashMap::new())),
            github,
        }
    }
}

// =============================================================================
// TEST HELPERS
// =============================================================================

#[cfg(test)]
#[path = "state_helpers_test.rs"]
pub mod test_helpers;

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
