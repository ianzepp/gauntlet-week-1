//! Persistence service — background flush for dirty objects.
//!
//! DESIGN
//! ======
//! A background task flushes dirty objects, then sleeps 100ms before
//! the next cycle. Frames use a bounded queue + batched async writer so
//! websocket handling never blocks on Postgres I/O.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tracing::{error, info, warn};

use crate::frame::Frame;
use crate::state::{AppState, BoardObject};

const DEFAULT_FRAME_PERSIST_QUEUE_CAPACITY: usize = 8192;
const DEFAULT_FRAME_PERSIST_BATCH_SIZE: usize = 128;
const DEFAULT_FRAME_PERSIST_FLUSH_MS: u64 = 5;
const DEFAULT_FRAME_PERSIST_RETRIES: usize = 2;
const DEFAULT_FRAME_PERSIST_RETRY_BASE_MS: u64 = 20;
const DEFAULT_OBJECT_FLUSH_INTERVAL_MS: u64 = 100;

#[derive(Clone, Copy)]
pub(crate) struct FramePersistConfig {
    pub(crate) queue_capacity: usize,
    pub(crate) batch_size: usize,
    pub(crate) flush_ms: u64,
    pub(crate) retries: usize,
    pub(crate) retry_base_ms: u64,
}

impl FramePersistConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            queue_capacity: env_parse("FRAME_PERSIST_QUEUE_CAPACITY", DEFAULT_FRAME_PERSIST_QUEUE_CAPACITY),
            batch_size: env_parse("FRAME_PERSIST_BATCH_SIZE", DEFAULT_FRAME_PERSIST_BATCH_SIZE),
            flush_ms: env_parse("FRAME_PERSIST_FLUSH_MS", DEFAULT_FRAME_PERSIST_FLUSH_MS),
            retries: env_parse("FRAME_PERSIST_RETRIES", DEFAULT_FRAME_PERSIST_RETRIES),
            retry_base_ms: env_parse("FRAME_PERSIST_RETRY_BASE_MS", DEFAULT_FRAME_PERSIST_RETRY_BASE_MS),
        }
    }
}

pub(crate) fn env_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

/// Spawn the background persistence task. Returns a handle for shutdown.
pub fn spawn_persistence_task(state: AppState) -> JoinHandle<()> {
    let flush_interval_ms = env_parse("OBJECT_FLUSH_INTERVAL_MS", DEFAULT_OBJECT_FLUSH_INTERVAL_MS);
    info!(flush_interval_ms, "object persistence flush configured");
    tokio::spawn(async move {
        loop {
            flush_all_dirty(&state).await;
            tokio::time::sleep(Duration::from_millis(flush_interval_ms)).await;
        }
    })
}

/// Spawn a bounded frame persistence worker and return its queue sender.
///
/// Frames are written in batches to reduce DB overhead and keep websocket
/// request/response latency predictable.
#[must_use]
pub fn spawn_frame_persistence_worker(pool: PgPool) -> tokio::sync::mpsc::Sender<Frame> {
    let config = FramePersistConfig::from_env();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Frame>(config.queue_capacity);

    info!(
        queue_capacity = config.queue_capacity,
        batch_size = config.batch_size,
        flush_ms = config.flush_ms,
        retries = config.retries,
        retry_base_ms = config.retry_base_ms,
        "frame persistence worker configured"
    );

    tokio::spawn(async move {
        let mut batch: Vec<Frame> = Vec::with_capacity(config.batch_size);
        let mut ticker = tokio::time::interval(Duration::from_millis(config.flush_ms));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                maybe_frame = rx.recv() => {
                    if let Some(frame) = maybe_frame {
                        batch.push(frame);
                        if batch.len() >= config.batch_size {
                            flush_frame_batch_with_retry(&pool, &mut batch, config).await;
                        }
                    } else {
                        flush_frame_batch_with_retry(&pool, &mut batch, config).await;
                        break;
                    }
                }
                _ = ticker.tick() => {
                    flush_frame_batch_with_retry(&pool, &mut batch, config).await;
                }
            }
        }
    });

    tx
}

/// Best-effort, non-blocking enqueue for frame persistence.
///
/// Uses `try_send` to avoid adding latency on websocket request handling.
pub fn enqueue_frame(state: &AppState, frame: &Frame) {
    let Some(tx) = &state.frame_persist_tx else {
        return;
    };

    match tx.try_send(frame.clone()) {
        Ok(()) => {}
        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
            warn!(id = %frame.id, syscall = %frame.syscall, "frame persist queue full; dropping frame");
        }
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
            warn!(id = %frame.id, syscall = %frame.syscall, "frame persist queue closed; dropping frame");
        }
    }
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
}

async fn flush_frame_batch_with_retry(pool: &PgPool, batch: &mut Vec<Frame>, config: FramePersistConfig) {
    if batch.is_empty() {
        return;
    }

    let drained = std::mem::take(batch);
    for attempt in 1..=config.retries {
        match persist_frame_batch(pool, &drained).await {
            Ok(()) => return,
            Err(e) if attempt < config.retries => {
                warn!(
                    error = %e,
                    attempt,
                    total = config.retries,
                    count = drained.len(),
                    "frame batch persist failed; retrying"
                );
                tokio::time::sleep(Duration::from_millis((attempt as u64) * config.retry_base_ms)).await;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    count = drained.len(),
                    "frame batch persist failed after retries; dropping frames"
                );
                return;
            }
        }
    }
}

/// Persist a single frame row.
pub async fn persist_frame(pool: &PgPool, frame: &Frame) -> Result<(), sqlx::Error> {
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
    Ok(())
}

/// Persist a batch of frames.
pub async fn persist_frame_batch(pool: &PgPool, frames: &[Frame]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
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
        .execute(tx.as_mut())
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "persistence_test.rs"]
mod tests;
