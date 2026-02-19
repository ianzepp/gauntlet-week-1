//! Savepoint service — point-in-time board snapshots for rewind workflows.
//!
//! DESIGN
//! ======
//! Savepoints store a full board snapshot and the current global frame sequence
//! for the board. This gives fast "rewind from checkpoint + replay tail" later
//! without turning every operation into a heavyweight snapshot write.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::board::{self, BoardPermission};
use crate::state::{AppState, BoardObject};

const DEFAULT_AUTO_SAVEPOINT_DEBOUNCE_MS: i64 = 1500;

#[derive(Debug, thiserror::Error)]
pub enum SavepointError {
    #[error("board not found or not accessible: {0}")]
    BoardNotFound(Uuid),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl crate::frame::ErrorCode for SavepointError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::BoardNotFound(_) => "E_BOARD_NOT_FOUND",
            Self::Database(_) => "E_DATABASE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavepointRow {
    pub id: Uuid,
    pub board_id: Uuid,
    pub seq: i64,
    pub ts: i64,
    pub created_by: Option<Uuid>,
    pub is_auto: bool,
    pub reason: String,
    pub label: Option<String>,
    pub snapshot: serde_json::Value,
}

fn now_ms() -> i64 {
    let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(dur.as_millis()).unwrap_or(0)
}

fn auto_savepoint_debounce_ms() -> i64 {
    std::env::var("AUTO_SAVEPOINT_DEBOUNCE_MS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_AUTO_SAVEPOINT_DEBOUNCE_MS)
}

async fn ensure_board_access(pool: &PgPool, board_id: Uuid, user_id: Uuid) -> Result<(), SavepointError> {
    board::ensure_board_permission(pool, board_id, user_id, BoardPermission::Edit)
        .await
        .map_err(|_| SavepointError::BoardNotFound(board_id))
}

async fn snapshot_objects(state: &AppState, board_id: Uuid) -> Result<Vec<BoardObject>, SavepointError> {
    {
        let boards = state.boards.read().await;
        if let Some(board_state) = boards.get(&board_id) {
            return Ok(board_state.objects.values().cloned().collect());
        }
    }

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
    .fetch_all(&state.pool)
    .await?;

    let mut objects = Vec::with_capacity(rows.len());
    for (id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version) in rows {
        objects.push(BoardObject {
            id,
            board_id,
            kind,
            x,
            y,
            width,
            height,
            rotation,
            z_index,
            props,
            created_by,
            version,
        });
    }
    Ok(objects)
}

async fn current_board_seq(pool: &PgPool, board_id: Uuid) -> Result<i64, SavepointError> {
    let seq: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM frames WHERE board_id = $1")
        .bind(board_id)
        .fetch_one(pool)
        .await?;
    Ok(seq.unwrap_or(0))
}

pub async fn create_savepoint(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
    label: Option<&str>,
    is_auto: bool,
    reason: &str,
) -> Result<SavepointRow, SavepointError> {
    ensure_board_access(&state.pool, board_id, user_id).await?;
    let objects = snapshot_objects(state, board_id).await?;
    let snapshot = serde_json::to_value(objects).unwrap_or_else(|_| serde_json::json!([]));
    let seq = current_board_seq(&state.pool, board_id).await?;

    let row = SavepointRow {
        id: Uuid::new_v4(),
        board_id,
        seq,
        ts: now_ms(),
        created_by: Some(user_id),
        is_auto,
        reason: reason.to_owned(),
        label: label.map(std::string::ToString::to_string),
        snapshot,
    };

    sqlx::query(
        "INSERT INTO board_savepoints (id, board_id, seq, ts, created_by, is_auto, reason, label, snapshot)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(row.id)
    .bind(row.board_id)
    .bind(row.seq)
    .bind(row.ts)
    .bind(row.created_by)
    .bind(row.is_auto)
    .bind(&row.reason)
    .bind(&row.label)
    .bind(&row.snapshot)
    .execute(&state.pool)
    .await?;

    Ok(row)
}

pub async fn list_savepoints(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<SavepointRow>, SavepointError> {
    ensure_board_access(&state.pool, board_id, user_id).await?;
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            i64,
            i64,
            Option<Uuid>,
            bool,
            String,
            Option<String>,
            serde_json::Value,
        ),
    >(
        "SELECT id, board_id, seq, ts, created_by, is_auto, reason, label, snapshot
         FROM board_savepoints
         WHERE board_id = $1
         ORDER BY seq DESC
         LIMIT 200",
    )
    .bind(board_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, board_id, seq, ts, created_by, is_auto, reason, label, snapshot)| SavepointRow {
                id,
                board_id,
                seq,
                ts,
                created_by,
                is_auto,
                reason,
                label,
                snapshot,
            },
        )
        .collect())
}

pub async fn maybe_create_auto_savepoint(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
    reason: &str,
) -> Result<Option<SavepointRow>, SavepointError> {
    ensure_board_access(&state.pool, board_id, user_id).await?;
    let latest_auto_ts: Option<i64> = sqlx::query_scalar(
        "SELECT ts
         FROM board_savepoints
         WHERE board_id = $1 AND is_auto = true
         ORDER BY seq DESC
         LIMIT 1",
    )
    .bind(board_id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();

    let now = now_ms();
    if let Some(ts) = latest_auto_ts
        && now.saturating_sub(ts) < auto_savepoint_debounce_ms()
    {
        return Ok(None);
    }

    let row = create_savepoint(state, board_id, user_id, Some("Auto savepoint"), true, reason).await?;
    Ok(Some(row))
}

pub fn savepoint_row_to_json(row: SavepointRow) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("id".into(), serde_json::json!(row.id));
    map.insert("board_id".into(), serde_json::json!(row.board_id));
    map.insert("seq".into(), serde_json::json!(row.seq));
    map.insert("ts".into(), serde_json::json!(row.ts));
    map.insert("is_auto".into(), serde_json::json!(row.is_auto));
    map.insert("reason".into(), serde_json::json!(row.reason));
    map.insert("label".into(), serde_json::json!(row.label));
    map.insert("created_by".into(), serde_json::json!(row.created_by));
    map.insert("snapshot".into(), row.snapshot);
    serde_json::Value::Object(map)
}

pub fn savepoint_rows_to_json(rows: Vec<SavepointRow>) -> Vec<serde_json::Value> {
    rows.into_iter().map(savepoint_row_to_json).collect()
}
