//! Persistence service — debounced 100ms flush of dirty objects and frames.
//!
//! DESIGN
//! ======
//! A background task wakes every 100ms, collects dirty objects from
//! all boards, clears the dirty sets, releases the lock, then batch
//! upserts to Postgres. This keeps the hot path (in-memory mutations)
//! fast while ensuring durability within a 100ms window.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;
use tracing::error;

use crate::frame::Frame;
use crate::state::{AppState, BoardObject};

/// Spawn the background persistence task. Returns a handle for shutdown.
pub fn spawn_persistence_task(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            flush_all_dirty(&state).await;
        }
    })
}

async fn flush_all_dirty(state: &AppState) {
    // Collect dirty objects under the lock, then release.
    let dirty_objects = {
        let mut boards = state.boards.write().await;
        let mut all_dirty = Vec::new();

        for board_state in boards.values_mut() {
            if board_state.dirty.is_empty() {
                continue;
            }

            let objects: Vec<BoardObject> = board_state
                .dirty
                .iter()
                .filter_map(|id| board_state.objects.get(id).cloned())
                .collect();

            board_state.dirty.clear();
            all_dirty.extend(objects);
        }

        all_dirty
    };

    if !dirty_objects.is_empty() {
        if let Err(e) = crate::services::board::flush_objects(&state.pool, &dirty_objects).await {
            error!(error = %e, count = dirty_objects.len(), "persistence flush failed");
        }
    }

    // Drain buffered frames and flush to DB.
    let dirty_frames: Vec<Frame> = {
        let Ok(mut buf) = state.dirty_frames.lock() else {
            return;
        };
        std::mem::take(&mut *buf)
    };

    if !dirty_frames.is_empty() {
        if let Err(e) = flush_frames(&state.pool, &dirty_frames).await {
            error!(error = %e, count = dirty_frames.len(), "frame persistence flush failed");
        }
    }
}

async fn flush_frames(pool: &PgPool, frames: &[Frame]) -> Result<(), sqlx::Error> {
    for frame in frames {
        let status = serde_json::to_value(frame.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        let data = serde_json::to_value(&frame.data).unwrap_or_default();

        sqlx::query(
            r#"INSERT INTO frames (id, parent_id, syscall, status, board_id, "from", data, ts)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(frame.id)
        .bind(frame.parent_id)
        .bind(&frame.syscall)
        .bind(&status)
        .bind(frame.board_id)
        .bind(&frame.from)
        .bind(&data)
        .bind(frame.ts)
        .execute(pool)
        .await?;
    }
    Ok(())
}
