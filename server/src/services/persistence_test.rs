use super::*;
use crate::state::test_helpers;
use crate::state::{BoardObject, BoardState};
use uuid::Uuid;

// =============================================================================
// env_parse
// =============================================================================

#[test]
fn env_parse_missing_returns_default() {
    let val: usize = env_parse("__TEST_NONEXISTENT_KEY_12345__", 42);
    assert_eq!(val, 42);
}

#[test]
fn env_parse_present_valid() {
    unsafe { std::env::set_var("__TEST_EP_VALID__", "99") };
    let val: usize = env_parse("__TEST_EP_VALID__", 0);
    assert_eq!(val, 99);
    unsafe { std::env::remove_var("__TEST_EP_VALID__") };
}

#[test]
fn env_parse_present_invalid_returns_default() {
    unsafe { std::env::set_var("__TEST_EP_INVALID__", "notanumber") };
    let val: usize = env_parse("__TEST_EP_INVALID__", 7);
    assert_eq!(val, 7);
    unsafe { std::env::remove_var("__TEST_EP_INVALID__") };
}

// =============================================================================
// FramePersistConfig defaults
// =============================================================================

#[test]
fn frame_persist_config_defaults_match_constants() {
    unsafe {
        std::env::remove_var("FRAME_PERSIST_QUEUE_CAPACITY");
        std::env::remove_var("FRAME_PERSIST_BATCH_SIZE");
        std::env::remove_var("FRAME_PERSIST_FLUSH_MS");
        std::env::remove_var("FRAME_PERSIST_RETRIES");
        std::env::remove_var("FRAME_PERSIST_RETRY_BASE_MS");
    }
    let config = FramePersistConfig::from_env();
    assert_eq!(config.queue_capacity, DEFAULT_FRAME_PERSIST_QUEUE_CAPACITY);
    assert_eq!(config.batch_size, DEFAULT_FRAME_PERSIST_BATCH_SIZE);
    assert_eq!(config.flush_ms, DEFAULT_FRAME_PERSIST_FLUSH_MS);
    assert_eq!(config.retries, DEFAULT_FRAME_PERSIST_RETRIES);
    assert_eq!(config.retry_base_ms, DEFAULT_FRAME_PERSIST_RETRY_BASE_MS);
}

// =============================================================================
// enqueue_frame — these need tokio context because test_app_state uses PgPool
// =============================================================================

#[tokio::test]
async fn enqueue_frame_no_sender_is_noop() {
    let state = test_helpers::test_app_state();
    assert!(state.frame_persist_tx.is_none());
    let frame = Frame::request("test:noop", crate::frame::Data::new());
    enqueue_frame(&state, &frame);
}

#[tokio::test]
async fn enqueue_frame_sends_to_channel() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Frame>(16);
    let mut state = test_helpers::test_app_state();
    state.frame_persist_tx = Some(tx);

    let frame = Frame::request("test:send", crate::frame::Data::new());
    enqueue_frame(&state, &frame);

    let received = rx.try_recv().unwrap();
    assert_eq!(received.id, frame.id);
    assert_eq!(received.syscall, "test:send");
}

#[tokio::test]
async fn enqueue_frame_full_channel_drops_frame() {
    let (tx, _rx) = tokio::sync::mpsc::channel::<Frame>(1);
    let mut state = test_helpers::test_app_state();
    state.frame_persist_tx = Some(tx);

    let f1 = Frame::request("test:fill", crate::frame::Data::new());
    let f2 = Frame::request("test:overflow", crate::frame::Data::new());

    enqueue_frame(&state, &f1);
    // Channel is full (capacity 1), second enqueue should not panic.
    enqueue_frame(&state, &f2);
}

#[tokio::test]
async fn enqueue_frame_closed_channel_drops_frame() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Frame>(16);
    let mut state = test_helpers::test_app_state();
    state.frame_persist_tx = Some(tx);

    drop(rx);

    let frame = Frame::request("test:closed", crate::frame::Data::new());
    enqueue_frame(&state, &frame);
}

#[tokio::test]
async fn flush_all_dirty_failure_preserves_dirty_flags() {
    let state = test_helpers::test_app_state();
    let board_id = Uuid::new_v4();
    let object = BoardObject {
        id: Uuid::new_v4(),
        board_id,
        kind: "sticky_note".to_owned(),
        x: 1.0,
        y: 2.0,
        width: None,
        height: None,
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({"text":"persist me"}),
        created_by: None,
        version: 1,
        group_id: None,
    };
    let object_id = object.id;

    let mut board_state = BoardState::new();
    board_state.objects.insert(object_id, object);
    board_state.dirty.insert(object_id);
    {
        let mut boards = state.boards.write().await;
        boards.insert(board_id, board_state);
    }

    // Test state uses connect_lazy; flush attempts fail and must not clear dirty flags.
    flush_all_dirty_for_tests(&state).await;

    let boards = state.boards.read().await;
    let board_state = boards.get(&board_id).expect("board should remain loaded");
    assert!(board_state.dirty.contains(&object_id));
}
