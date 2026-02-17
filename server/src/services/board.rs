//! Board service — CRUD, join/part, and state hydration.
//!
//! DESIGN
//! ======
//! Boards are created and listed via REST-like operations (dispatched from
//! WS frames). Board state is hydrated from Postgres on first join and kept
//! in memory while any client is connected.

use std::collections::HashMap;

use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use crate::frame::Frame;
use crate::state::{AppState, BoardObject, BoardState};

// =============================================================================
// TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum BoardError {
    #[error("board not found: {0}")]
    NotFound(Uuid),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl crate::frame::ErrorCode for BoardError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "E_BOARD_NOT_FOUND",
            Self::Database(_) => "E_DATABASE",
        }
    }
}

/// Row returned from board queries.
#[derive(Debug, Clone)]
pub struct BoardRow {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
}

// =============================================================================
// CRUD
// =============================================================================

/// Create a new board.
///
/// # Errors
///
/// Returns a database error if the insert fails.
pub async fn create_board(pool: &PgPool, name: &str) -> Result<BoardRow, BoardError> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO boards (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await?;

    Ok(BoardRow { id, name: name.to_string(), owner_id: None })
}

/// List all boards.
///
/// # Errors
///
/// Returns a database error if the query fails.
pub async fn list_boards(pool: &PgPool) -> Result<Vec<BoardRow>, BoardError> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<Uuid>)>(
        "SELECT id, name, owner_id FROM boards ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, owner_id)| BoardRow { id, name, owner_id })
        .collect())
}

/// Delete a board by ID.
///
/// # Errors
///
/// Returns a database error if the delete fails.
pub async fn delete_board(pool: &PgPool, board_id: Uuid) -> Result<(), BoardError> {
    let result = sqlx::query("DELETE FROM boards WHERE id = $1")
        .bind(board_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BoardError::NotFound(board_id));
    }
    Ok(())
}

// =============================================================================
// JOIN / PART
// =============================================================================

/// Join a board. Hydrates from Postgres if not already in memory.
/// Returns the current list of board objects.
///
/// # Errors
///
/// Returns a database error if hydration fails.
pub async fn join_board(
    state: &AppState,
    board_id: Uuid,
    client_id: Uuid,
    tx: mpsc::Sender<Frame>,
) -> Result<Vec<BoardObject>, BoardError> {
    // Verify board exists in database.
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM boards WHERE id = $1)")
        .bind(board_id)
        .fetch_one(&state.pool)
        .await?;

    if !exists {
        return Err(BoardError::NotFound(board_id));
    }

    let mut boards = state.boards.write().await;
    let board_state = boards.entry(board_id).or_insert_with(BoardState::new);

    // Hydrate from Postgres if this is the first client.
    if board_state.clients.is_empty() {
        let objects = hydrate_objects(&state.pool, board_id).await?;
        board_state.objects = objects;
        info!(%board_id, count = board_state.objects.len(), "hydrated board from database");
    }

    board_state.clients.insert(client_id, tx);
    let objects: Vec<BoardObject> = board_state.objects.values().cloned().collect();

    info!(%board_id, %client_id, clients = board_state.clients.len(), "client joined board");
    Ok(objects)
}

/// Leave a board. Removes the client sender. If last client, flushes
/// dirty objects and evicts the board state from memory.
pub async fn part_board(state: &AppState, board_id: Uuid, client_id: Uuid) {
    let mut boards = state.boards.write().await;
    let Some(board_state) = boards.get_mut(&board_id) else {
        return;
    };

    board_state.clients.remove(&client_id);
    info!(%board_id, %client_id, remaining = board_state.clients.len(), "client left board");

    if board_state.clients.is_empty() {
        // Final flush of dirty objects before eviction.
        if board_state.dirty.is_empty() {
            boards.remove(&board_id);
            info!(%board_id, "evicted board from memory");
        } else {
            let dirty_objects: Vec<BoardObject> = board_state
                .dirty
                .iter()
                .filter_map(|id| board_state.objects.get(id).cloned())
                .collect();
            board_state.dirty.clear();

            // Release lock before writing to Postgres.
            drop(boards);
            if let Err(e) = flush_objects(&state.pool, &dirty_objects).await {
                tracing::error!(error = %e, %board_id, "final flush failed");
            }

            // Re-acquire and evict.
            let mut boards = state.boards.write().await;
            if let Some(bs) = boards.get(&board_id) {
                if bs.clients.is_empty() {
                    boards.remove(&board_id);
                    info!(%board_id, "evicted board from memory");
                }
            }
        }
    }
}

// =============================================================================
// BROADCAST
// =============================================================================

/// Broadcast a frame to all clients in a board, optionally excluding one.
pub async fn broadcast(state: &AppState, board_id: Uuid, frame: &Frame, exclude: Option<Uuid>) {
    let boards = state.boards.read().await;
    let Some(board_state) = boards.get(&board_id) else {
        return;
    };

    for (client_id, tx) in &board_state.clients {
        if exclude == Some(*client_id) {
            continue;
        }
        // Best-effort: if a client's channel is full, skip it.
        let _ = tx.try_send(frame.clone());
    }
}

// =============================================================================
// HELPERS
// =============================================================================

async fn hydrate_objects(pool: &PgPool, board_id: Uuid) -> Result<HashMap<Uuid, BoardObject>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            f64,
            i32,
            serde_json::Value,
            Option<Uuid>,
            i32,
        ),
    >(
        "SELECT id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version \
         FROM board_objects WHERE board_id = $1",
    )
    .bind(board_id)
    .fetch_all(pool)
    .await?;

    let mut objects = HashMap::new();
    for (id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version) in rows {
        objects.insert(
            id,
            BoardObject { id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version },
        );
    }
    Ok(objects)
}

/// Batch upsert objects to Postgres.
pub async fn flush_objects(pool: &PgPool, objects: &[BoardObject]) -> Result<(), sqlx::Error> {
    for obj in objects {
        sqlx::query(
            "INSERT INTO board_objects (id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now()) \
             ON CONFLICT (id) DO UPDATE SET \
                 x = EXCLUDED.x, y = EXCLUDED.y, width = EXCLUDED.width, height = EXCLUDED.height, \
                 rotation = EXCLUDED.rotation, z_index = EXCLUDED.z_index, props = EXCLUDED.props, \
                 version = EXCLUDED.version, updated_at = now()",
        )
        .bind(obj.id)
        .bind(obj.board_id)
        .bind(&obj.kind)
        .bind(obj.x)
        .bind(obj.y)
        .bind(obj.width)
        .bind(obj.height)
        .bind(obj.rotation)
        .bind(obj.z_index)
        .bind(&obj.props)
        .bind(obj.created_by)
        .bind(obj.version)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "board_test.rs"]
mod tests;
