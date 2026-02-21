//! Persistence service — background flush for dirty objects.
//!
//! DESIGN
//! ======
//! A background task flushes dirty objects, then sleeps 100ms before
//! the next cycle. Frames use a bounded queue + batched async writer so
//! websocket handling never blocks on Postgres I/O.
//!
//! ERROR HANDLING
//! ==============
//! Dirty flags are cleared only after successful writes. This prioritizes
//! durability over duplicate flush attempts: repeated upserts are acceptable,
//! silent data loss is not.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tracing::{error, info, warn};

use crate::frame::Frame;
use crate::state::{AppState, BoardObject};
use uuid::Uuid;

const DEFAULT_FRAME_PERSIST_QUEUE_CAPACITY: usize = 8192;
const DEFAULT_FRAME_PERSIST_BATCH_SIZE: usize = 128;
const DEFAULT_FRAME_PERSIST_FLUSH_MS: u64 = 5;
const DEFAULT_FRAME_PERSIST_RETRIES: usize = 2;
const DEFAULT_FRAME_PERSIST_RETRY_BASE_MS: u64 = 20;
const DEFAULT_OBJECT_FLUSH_INTERVAL_MS: u64 = 100;

/// Tuning knobs for the frame persistence worker, loaded from environment variables.
#[derive(Clone, Copy)]
pub(crate) struct FramePersistConfig {
    /// Bounded channel capacity for the frame persist queue.
    pub(crate) queue_capacity: usize,
    /// Maximum frames flushed per Postgres write batch.
    pub(crate) batch_size: usize,
    /// How long to wait for the batch to fill before flushing, in milliseconds.
    pub(crate) flush_ms: u64,
    /// Number of retry attempts on transient database failures.
    pub(crate) retries: usize,
    /// Base delay in milliseconds for exponential retry back-off.
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
    // PHASE: SNAPSHOT DIRTY OBJECTS
    // WHY: collect immutable clones under lock, then perform I/O lock-free.
    let batches = {
        let mut boards = state.boards.write().await;
        let mut collected = Vec::new();

        for (board_id, board_state) in boards.iter_mut() {
            if board_state.dirty.is_empty() {
                continue;
            }

            let objects = board_state
                .dirty
                .iter()
                .filter_map(|id| board_state.objects.get(id).cloned())
                .collect::<Vec<_>>();
            if objects.is_empty() {
                continue;
            }
            let versions = objects
                .iter()
                .map(|obj| (obj.id, obj.version))
                .collect::<Vec<_>>();
            collected.push(DirtyFlushBatch { board_id: *board_id, objects, flushed_versions: versions });
        }

        collected
    };

    // PHASE: FLUSH PER BOARD + ACK DIRTY IDS
    // WHY: if flush fails we intentionally keep dirty flags for retry.
    for batch in batches {
        match crate::services::board::flush_objects(&state.pool, &batch.objects).await {
            Ok(()) => {
                clear_flushed_dirty_ids(state, batch.board_id, &batch.flushed_versions).await;
            }
            Err(e) => {
                error!(error = %e, count = batch.objects.len(), board_id = %batch.board_id, "persistence flush failed");
            }
        }
    }
}

#[cfg(test)]
pub(crate) async fn flush_all_dirty_for_tests(state: &AppState) {
    flush_all_dirty(state).await;
}

#[derive(Debug)]
struct DirtyFlushBatch {
    board_id: Uuid,
    objects: Vec<BoardObject>,
    flushed_versions: Vec<(Uuid, i32)>,
}

async fn clear_flushed_dirty_ids(state: &AppState, board_id: Uuid, flushed_versions: &[(Uuid, i32)]) {
    let mut boards = state.boards.write().await;
    let Some(board_state) = boards.get_mut(&board_id) else {
        return;
    };

    for (object_id, flushed_version) in flushed_versions {
        // EDGE: keep dirty flag if object was updated again after snapshot.
        let can_clear = match board_state.objects.get(object_id) {
            Some(current) => current.version == *flushed_version,
            None => true,
        };
        if can_clear {
            board_state.dirty.remove(object_id);
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
