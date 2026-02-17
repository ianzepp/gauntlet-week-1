//! Object service — create, update, delete with LWW versioning.
//!
//! DESIGN
//! ======
//! Object mutations update in-memory state immediately, mark the object
//! as dirty for debounced persistence, and return the updated object for
//! broadcast. LWW conflict resolution: incoming version must be >= current
//! version, otherwise the update is rejected as stale.

use uuid::Uuid;

use crate::frame::Data;
use crate::state::{AppState, BoardObject};

// =============================================================================
// TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ObjectError {
    #[error("object not found: {0}")]
    NotFound(Uuid),
    #[error("board not loaded: {0}")]
    BoardNotLoaded(Uuid),
    #[error("stale update: incoming version {incoming} < current {current}")]
    StaleUpdate { incoming: i32, current: i32 },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl crate::frame::ErrorCode for ObjectError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "E_OBJECT_NOT_FOUND",
            Self::BoardNotLoaded(_) => "E_BOARD_NOT_LOADED",
            Self::StaleUpdate { .. } => "E_STALE_UPDATE",
            Self::Database(_) => "E_DATABASE",
        }
    }
}

// =============================================================================
// CREATE
// =============================================================================

/// Create a new object on a board.
///
/// # Errors
///
/// Returns `BoardNotLoaded` if the board isn't in memory.
pub async fn create_object(
    state: &AppState,
    board_id: Uuid,
    kind: &str,
    x: f64,
    y: f64,
    props: serde_json::Value,
    created_by: Option<Uuid>,
) -> Result<BoardObject, ObjectError> {
    let mut boards = state.boards.write().await;
    let board = boards
        .get_mut(&board_id)
        .ok_or(ObjectError::BoardNotLoaded(board_id))?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let z_index = board.objects.len() as i32;
    let obj = BoardObject {
        id: Uuid::new_v4(),
        board_id,
        kind: kind.to_string(),
        x,
        y,
        width: None,
        height: None,
        rotation: 0.0,
        z_index,
        props,
        created_by,
        version: 1,
    };

    let result = obj.clone();
    board.dirty.insert(obj.id);
    board.objects.insert(obj.id, obj);

    Ok(result)
}

// =============================================================================
// UPDATE
// =============================================================================

/// Update an existing object with LWW conflict resolution.
///
/// # Errors
///
/// Returns `StaleUpdate` if `incoming_version < current.version`.
pub async fn update_object(
    state: &AppState,
    board_id: Uuid,
    object_id: Uuid,
    updates: &Data,
    incoming_version: i32,
) -> Result<BoardObject, ObjectError> {
    let mut boards = state.boards.write().await;
    let board = boards
        .get_mut(&board_id)
        .ok_or(ObjectError::BoardNotLoaded(board_id))?;
    let obj = board
        .objects
        .get_mut(&object_id)
        .ok_or(ObjectError::NotFound(object_id))?;

    // LWW: reject stale updates.
    if incoming_version < obj.version {
        return Err(ObjectError::StaleUpdate { incoming: incoming_version, current: obj.version });
    }

    // Apply updates from data map.
    if let Some(x) = updates.get("x").and_then(serde_json::Value::as_f64) {
        obj.x = x;
    }
    if let Some(y) = updates.get("y").and_then(serde_json::Value::as_f64) {
        obj.y = y;
    }
    if let Some(w) = updates.get("width").and_then(serde_json::Value::as_f64) {
        obj.width = Some(w);
    }
    if let Some(h) = updates.get("height").and_then(serde_json::Value::as_f64) {
        obj.height = Some(h);
    }
    if let Some(r) = updates.get("rotation").and_then(serde_json::Value::as_f64) {
        obj.rotation = r;
    }
    if let Some(z) = updates.get("z_index").and_then(serde_json::Value::as_i64) {
        #[allow(clippy::cast_possible_truncation)]
        {
            obj.z_index = z as i32;
        }
    }
    if let Some(p) = updates.get("props") {
        obj.props = p.clone();
    }

    obj.version += 1;
    board.dirty.insert(object_id);

    Ok(obj.clone())
}

// =============================================================================
// DELETE
// =============================================================================

/// Delete an object from a board. Removes from memory and Postgres immediately.
///
/// # Errors
///
/// Returns `NotFound` if the object doesn't exist.
pub async fn delete_object(state: &AppState, board_id: Uuid, object_id: Uuid) -> Result<(), ObjectError> {
    let mut boards = state.boards.write().await;
    let board = boards
        .get_mut(&board_id)
        .ok_or(ObjectError::BoardNotLoaded(board_id))?;

    if board.objects.remove(&object_id).is_none() {
        return Err(ObjectError::NotFound(object_id));
    }
    board.dirty.remove(&object_id);

    // Delete from Postgres immediately (not deferred).
    sqlx::query("DELETE FROM board_objects WHERE id = $1")
        .bind(object_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}

#[cfg(test)]
#[path = "object_test.rs"]
mod tests;
