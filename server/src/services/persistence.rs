//! Persistence service — debounced 1-second flush of dirty objects.
//!
//! DESIGN
//! ======
//! A background task wakes every 1 second, collects dirty objects from
//! all boards, clears the dirty sets, releases the lock, then batch
//! upserts to Postgres. This keeps the hot path (in-memory mutations)
//! fast while ensuring durability within a 1-second window.

use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::error;

use crate::state::{AppState, BoardObject};

/// Spawn the background persistence task. Returns a handle for shutdown.
pub fn spawn_persistence_task(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
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

    if dirty_objects.is_empty() {
        return;
    }

    if let Err(e) = crate::services::board::flush_objects(&state.pool, &dirty_objects).await {
        error!(error = %e, count = dirty_objects.len(), "persistence flush failed");
    }
}
