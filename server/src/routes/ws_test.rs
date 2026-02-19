use super::*;
use crate::frame::Status;
use crate::llm::types::{ChatResponse, ContentBlock, LlmChat, LlmError, Message, Tool};
use crate::state::test_helpers;
use serde_json::json;
#[cfg(feature = "live-db-tests")]
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};

struct MockLlm {
    responses: Mutex<Vec<ChatResponse>>,
}

impl MockLlm {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self { responses: Mutex::new(responses) }
    }
}

#[async_trait::async_trait]
impl LlmChat for MockLlm {
    async fn chat(
        &self,
        _max_tokens: u32,
        _system: &str,
        _messages: &[Message],
        _tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        let mut responses = self.responses.lock().expect("mock mutex should lock");
        if responses.is_empty() {
            Ok(ChatResponse {
                content: vec![ContentBlock::Text { text: "done".into() }],
                model: "mock".into(),
                stop_reason: "end_turn".into(),
                input_tokens: 0,
                output_tokens: 0,
            })
        } else {
            Ok(responses.remove(0))
        }
    }
}

fn ai_prompt_bytes(board_id: Uuid, prompt: &str) -> Vec<u8> {
    let mut data = Data::new();
    data.insert("prompt".into(), json!(prompt));
    let req = Frame::request("ai:prompt", data).with_board_id(board_id);
    frames::encode_frame(&frames::Frame::from(&req))
}

fn request_bytes(board_id: Uuid, syscall: &str, data: Data) -> Vec<u8> {
    let req = Frame::request(syscall, data).with_board_id(board_id);
    frames::encode_frame(&frames::Frame::from(&req))
}

async fn recv_board_broadcast(rx: &mut mpsc::Receiver<Frame>) -> Frame {
    timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("broadcast receive timed out")
        .expect("broadcast channel closed unexpectedly")
}

async fn recv_board_broadcasts(rx: &mut mpsc::Receiver<Frame>, count: usize) -> Vec<Frame> {
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        frames.push(recv_board_broadcast(rx).await);
    }
    frames
}

async fn assert_no_board_broadcast(rx: &mut mpsc::Receiver<Frame>) {
    assert!(
        timeout(Duration::from_millis(80), rx.recv()).await.is_err(),
        "expected no broadcast frame"
    );
}

#[tokio::test]
async fn board_list_refresh_broadcast_reaches_all_ws_clients() {
    let state = test_helpers::test_app_state();
    let client_a = Uuid::new_v4();
    let client_b = Uuid::new_v4();
    let (tx_a, mut rx_a) = mpsc::channel::<Frame>(8);
    let (tx_b, mut rx_b) = mpsc::channel::<Frame>(8);

    {
        let mut clients = state.ws_clients.write().await;
        clients.insert(client_a, tx_a);
        clients.insert(client_b, tx_b);
    }

    super::broadcast_board_list_refresh(&state).await;

    let a = timeout(Duration::from_millis(200), rx_a.recv())
        .await
        .expect("client A refresh timed out")
        .expect("client A channel closed");
    let b = timeout(Duration::from_millis(200), rx_b.recv())
        .await
        .expect("client B refresh timed out")
        .expect("client B channel closed");

    assert_eq!(a.syscall, "board:list:refresh");
    assert_eq!(a.status, Status::Request);
    assert_eq!(b.syscall, "board:list:refresh");
    assert_eq!(b.status, Status::Request);
}

async fn process_inbound_bytes(
    state: &AppState,
    current_board: &mut Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    client_tx: &mpsc::Sender<Frame>,
    bytes: &[u8],
) -> Vec<Frame> {
    let parsed = frames::decode_frame(bytes)
        .ok()
        .and_then(|wire| Frame::try_from(wire).ok());
    let user_name = parsed
        .as_ref()
        .and_then(|f| f.data.get("name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Test User");
    let user_color = parsed
        .as_ref()
        .and_then(|f| f.data.get("color"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("#8a8178");

    super::process_inbound_bytes(
        state,
        current_board,
        client_id,
        user_id,
        user_name,
        user_color,
        client_tx,
        bytes,
    )
    .await
}

#[cfg(feature = "live-db-tests")]
async fn integration_pool() -> sqlx::PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test:test@localhost:5432/test_gauntlet_week_1".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("requires reachable Postgres; set TEST_DATABASE_URL");

    sqlx::migrate!("src/db/migrations")
        .run(&pool)
        .await
        .expect("migrations should run");

    sqlx::query("TRUNCATE TABLE frames, board_objects, boards RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("test cleanup should succeed");

    pool
}

async fn register_two_clients(
    state: &AppState,
    board_id: Uuid,
) -> (
    Uuid,
    mpsc::Sender<Frame>,
    mpsc::Receiver<Frame>,
    Uuid,
    mpsc::Sender<Frame>,
    mpsc::Receiver<Frame>,
) {
    let sender_client_id = Uuid::new_v4();
    let peer_client_id = Uuid::new_v4();

    let (sender_tx, sender_rx) = mpsc::channel(32);
    let (peer_tx, peer_rx) = mpsc::channel(32);

    let mut boards = state.boards.write().await;
    let board = boards
        .get_mut(&board_id)
        .expect("board should exist in memory");
    board.clients.insert(sender_client_id, sender_tx.clone());
    board.clients.insert(peer_client_id, peer_tx.clone());

    (sender_client_id, sender_tx, sender_rx, peer_client_id, peer_tx, peer_rx)
}

#[tokio::test]
async fn board_join_requires_board_id() {
    let state = test_helpers::test_app_state();
    let (client_tx, _client_rx) = mpsc::channel(8);
    let mut current_board = None;

    let req = Frame::request("board:join", Data::new());
    let text = frames::encode_frame(&frames::Frame::from(&req));

    let reply =
        process_inbound_bytes(&state, &mut current_board, Uuid::new_v4(), Uuid::new_v4(), &client_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].syscall, "board:join");
    assert_eq!(reply[0].status, Status::Error);
    assert!(
        reply[0]
            .data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("board_id required")
    );
}

#[tokio::test]
async fn board_unknown_op_returns_error() {
    let state = test_helpers::test_app_state();
    let (client_tx, _client_rx) = mpsc::channel(8);
    let mut current_board = None;

    let req = Frame::request("board:not_a_real_op", Data::new()).with_board_id(Uuid::new_v4());
    let text = frames::encode_frame(&frames::Frame::from(&req));

    let reply =
        process_inbound_bytes(&state, &mut current_board, Uuid::new_v4(), Uuid::new_v4(), &client_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].syscall, "board:not_a_real_op");
    assert_eq!(reply[0].status, Status::Error);
    assert!(
        reply[0]
            .data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("unknown board op")
    );
}

#[tokio::test]
async fn chat_message_requires_joined_board() {
    let state = test_helpers::test_app_state();
    let (client_tx, mut client_rx) = mpsc::channel(8);
    let mut current_board = None;

    let mut data = Data::new();
    data.insert("message".into(), json!("hello"));
    let text = request_bytes(Uuid::new_v4(), "chat:message", data);

    let reply =
        process_inbound_bytes(&state, &mut current_board, Uuid::new_v4(), Uuid::new_v4(), &client_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].syscall, "chat:message");
    assert_eq!(reply[0].status, Status::Error);
    assert!(
        reply[0]
            .data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("must join a board first")
    );
    assert_no_board_broadcast(&mut client_rx).await;
}

#[tokio::test]
async fn chat_message_requires_non_empty_message() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let mut current_board = Some(board_id);

    let mut data = Data::new();
    data.insert("message".into(), json!("    "));
    let text = request_bytes(board_id, "chat:message", data);

    let reply =
        process_inbound_bytes(&state, &mut current_board, sender_client_id, Uuid::new_v4(), &sender_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].syscall, "chat:message");
    assert_eq!(reply[0].status, Status::Error);
    assert!(
        reply[0]
            .data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("message required")
    );
    assert_no_board_broadcast(&mut sender_rx).await;
    assert_no_board_broadcast(&mut peer_rx).await;
}

#[tokio::test]
async fn chat_message_broadcasts_to_peers_and_replies_with_trimmed_message() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let mut current_board = Some(board_id);
    let user_id = Uuid::new_v4();

    let mut data = Data::new();
    data.insert("message".into(), json!("  hello board  "));
    let text = request_bytes(board_id, "chat:message", data);

    let sender_frames =
        process_inbound_bytes(&state, &mut current_board, sender_client_id, user_id, &sender_tx, &text).await;

    assert_eq!(sender_frames.len(), 1);
    assert_eq!(sender_frames[0].syscall, "chat:message");
    assert_eq!(sender_frames[0].status, Status::Done);
    assert_eq!(
        sender_frames[0]
            .data
            .get("message")
            .and_then(|v| v.as_str()),
        Some("hello board")
    );
    let expected_from = user_id.to_string();
    assert_eq!(sender_frames[0].from.as_deref(), Some(expected_from.as_str()));

    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(peer_broadcast.syscall, "chat:message");
    assert_eq!(peer_broadcast.status, Status::Done);
    assert_eq!(peer_broadcast.data.get("message").and_then(|v| v.as_str()), Some("hello board"));
    assert!(peer_broadcast.parent_id.is_none());

    assert_no_board_broadcast(&mut sender_rx).await;
}

#[tokio::test]
async fn cursor_moved_broadcasts_to_peers_with_name_and_color() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let mut current_board = Some(board_id);
    let user_id = Uuid::new_v4();

    let mut data = Data::new();
    data.insert("x".into(), json!(321.5));
    data.insert("y".into(), json!(654.25));
    data.insert("camera_rotation".into(), json!(33.0));
    data.insert("name".into(), json!("Alice"));
    data.insert("color".into(), json!("#22c55e"));
    let text = request_bytes(board_id, "cursor:moved", data);

    let sender_frames =
        process_inbound_bytes(&state, &mut current_board, sender_client_id, user_id, &sender_tx, &text).await;

    // Cursor events are peer-only and do not echo a response frame to sender.
    assert!(sender_frames.is_empty());
    assert_no_board_broadcast(&mut sender_rx).await;

    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(peer_broadcast.syscall, "cursor:moved");
    let expected_client_id = sender_client_id.to_string();
    assert_eq!(
        peer_broadcast
            .data
            .get("client_id")
            .and_then(|v| v.as_str()),
        Some(expected_client_id.as_str())
    );
    assert_eq!(peer_broadcast.data.get("x").and_then(|v| v.as_f64()), Some(321.5));
    assert_eq!(peer_broadcast.data.get("y").and_then(|v| v.as_f64()), Some(654.25));
    assert_eq!(
        peer_broadcast
            .data
            .get("camera_rotation")
            .and_then(|v| v.as_f64()),
        Some(33.0)
    );
    assert_eq!(peer_broadcast.data.get("name").and_then(|v| v.as_str()), Some("Alice"));
    assert_eq!(peer_broadcast.data.get("color").and_then(|v| v.as_str()), Some("#22c55e"));
}

#[tokio::test]
async fn cursor_clear_broadcasts_to_peers() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let mut current_board = Some(board_id);
    let user_id = Uuid::new_v4();

    let text = request_bytes(board_id, "cursor:clear", Data::new());
    let sender_frames =
        process_inbound_bytes(&state, &mut current_board, sender_client_id, user_id, &sender_tx, &text).await;

    assert!(sender_frames.is_empty());
    assert_no_board_broadcast(&mut sender_rx).await;

    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(peer_broadcast.syscall, "cursor:clear");
    let expected_client_id = sender_client_id.to_string();
    assert_eq!(
        peer_broadcast
            .data
            .get("client_id")
            .and_then(|v| v.as_str()),
        Some(expected_client_id.as_str())
    );
}

#[tokio::test]
async fn object_drag_broadcasts_ephemeral_transform_to_peers() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let mut current_board = Some(board_id);
    let user_id = Uuid::new_v4();

    let mut data = Data::new();
    data.insert("id".into(), json!(Uuid::new_v4()));
    data.insert("x".into(), json!(120.0));
    data.insert("y".into(), json!(210.0));
    data.insert("width".into(), json!(180.0));
    data.insert("height".into(), json!(95.0));
    data.insert("rotation".into(), json!(15.0));
    let text = request_bytes(board_id, "object:drag", data);

    let sender_frames =
        process_inbound_bytes(&state, &mut current_board, sender_client_id, user_id, &sender_tx, &text).await;

    // Drag events are peer-only, no sender reply.
    assert!(sender_frames.is_empty());
    assert_no_board_broadcast(&mut sender_rx).await;

    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(peer_broadcast.syscall, "object:drag");
    assert_eq!(peer_broadcast.status, Status::Request);
    assert_eq!(peer_broadcast.data.get("x").and_then(|v| v.as_f64()), Some(120.0));
    assert_eq!(peer_broadcast.data.get("y").and_then(|v| v.as_f64()), Some(210.0));
    assert_eq!(peer_broadcast.data.get("width").and_then(|v| v.as_f64()), Some(180.0));
    assert_eq!(peer_broadcast.data.get("height").and_then(|v| v.as_f64()), Some(95.0));
    assert_eq!(peer_broadcast.data.get("rotation").and_then(|v| v.as_f64()), Some(15.0));
}

#[tokio::test]
async fn object_drag_end_broadcasts_to_peers_without_sender_reply() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let mut current_board = Some(board_id);
    let user_id = Uuid::new_v4();
    let object_id = Uuid::new_v4();

    let mut data = Data::new();
    data.insert("id".into(), json!(object_id));
    let text = request_bytes(board_id, "object:drag:end", data);

    let sender_frames =
        process_inbound_bytes(&state, &mut current_board, sender_client_id, user_id, &sender_tx, &text).await;

    assert!(sender_frames.is_empty());
    assert_no_board_broadcast(&mut sender_rx).await;

    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(peer_broadcast.syscall, "object:drag:end");
    assert_eq!(peer_broadcast.status, Status::Request);
    assert_eq!(
        peer_broadcast.data.get("id").and_then(|v| v.as_str()),
        Some(object_id.to_string().as_str())
    );
}

#[tokio::test]
async fn chat_history_requires_joined_board() {
    let state = test_helpers::test_app_state();
    let (client_tx, mut client_rx) = mpsc::channel(8);
    let mut current_board = None;
    let text = request_bytes(Uuid::new_v4(), "chat:history", Data::new());

    let reply =
        process_inbound_bytes(&state, &mut current_board, Uuid::new_v4(), Uuid::new_v4(), &client_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].syscall, "chat:history");
    assert_eq!(reply[0].status, Status::Error);
    assert!(
        reply[0]
            .data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("must join a board first")
    );
    assert_no_board_broadcast(&mut client_rx).await;
}

#[cfg(feature = "live-db-tests")]
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL/live Postgres"]
async fn chat_history_returns_persisted_messages_for_board() {
    let pool = integration_pool().await;
    let board = services::board::create_board(&pool, "Chat Board", Uuid::new_v4())
        .await
        .expect("create_board should succeed");
    let board_id = board.id;

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'chat:message', 'request', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(1_i64)
    .bind(board_id)
    .bind(user_a.to_string())
    .bind(json!({ "message": "first" }))
    .execute(&pool)
    .await
    .expect("insert first message should succeed");

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'chat:message', 'request', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(2_i64)
    .bind(board_id)
    .bind(user_b.to_string())
    .bind(json!({ "message": "second" }))
    .execute(&pool)
    .await
    .expect("insert second message should succeed");

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'object:update', 'request', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(3_i64)
    .bind(board_id)
    .bind(user_a.to_string())
    .bind(json!({ "id": Uuid::new_v4() }))
    .execute(&pool)
    .await
    .expect("insert non-chat frame should succeed");

    let state = AppState::new(pool, None, None);
    let (client_tx, _client_rx) = mpsc::channel(8);
    let mut current_board = Some(board_id);
    let text = request_bytes(board_id, "chat:history", Data::new());

    let reply =
        process_inbound_bytes(&state, &mut current_board, Uuid::new_v4(), Uuid::new_v4(), &client_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].status, Status::Done);
    assert_eq!(reply[0].syscall, "chat:history");
    let messages = reply[0]
        .data
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array should be present");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].get("message").and_then(|v| v.as_str()), Some("first"));
    assert_eq!(messages[1].get("message").and_then(|v| v.as_str()), Some("second"));
}

#[cfg(feature = "live-db-tests")]
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL/live Postgres"]
async fn ai_history_returns_only_messages_for_requesting_user() {
    let pool = integration_pool().await;
    let board = services::board::create_board(&pool, "AI History Board", Uuid::new_v4())
        .await
        .expect("create_board should succeed");
    let board_id = board.id;

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'ai:prompt', 'request', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(1_i64)
    .bind(board_id)
    .bind(user_a.to_string())
    .bind(json!({ "prompt": "user a prompt" }))
    .execute(&pool)
    .await
    .expect("insert user a request should succeed");

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'ai:prompt', 'done', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(2_i64)
    .bind(board_id)
    .bind(user_a.to_string())
    .bind(json!({ "text": "user a reply", "mutations": 1 }))
    .execute(&pool)
    .await
    .expect("insert user a reply should succeed");

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'ai:prompt', 'request', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(3_i64)
    .bind(board_id)
    .bind(user_b.to_string())
    .bind(json!({ "prompt": "user b prompt" }))
    .execute(&pool)
    .await
    .expect("insert user b request should succeed");

    sqlx::query(
        r#"INSERT INTO frames (id, ts, board_id, "from", syscall, status, data)
           VALUES ($1, $2, $3, $4, 'ai:prompt', 'done', $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(4_i64)
    .bind(board_id)
    .bind(user_b.to_string())
    .bind(json!({ "text": "user b reply", "mutations": 1 }))
    .execute(&pool)
    .await
    .expect("insert user b reply should succeed");

    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![]));
    let state = AppState::new(pool, Some(llm), None);
    let (client_tx, _client_rx) = mpsc::channel(8);
    let mut current_board = Some(board_id);
    let text = request_bytes(board_id, "ai:history", Data::new());

    let reply = process_inbound_bytes(&state, &mut current_board, Uuid::new_v4(), user_a, &client_tx, &text).await;

    assert_eq!(reply.len(), 1);
    assert_eq!(reply[0].status, Status::Done);
    assert_eq!(reply[0].syscall, "ai:history");
    let messages = reply[0]
        .data
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array should be present");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].get("role").and_then(|v| v.as_str()), Some("user"));
    assert_eq!(messages[0].get("text").and_then(|v| v.as_str()), Some("user a prompt"));
    assert_eq!(messages[1].get("role").and_then(|v| v.as_str()), Some("assistant"));
    assert_eq!(messages[1].get("text").and_then(|v| v.as_str()), Some("user a reply"));
}

#[tokio::test]
async fn multi_user_single_change_reaches_other_user() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (client_a_id, client_a_tx, mut client_a_rx, _client_b_id, _client_b_tx, mut client_b_rx) =
        register_two_clients(&state, board_id).await;

    let mut current_board_a = Some(board_id);
    let user_a = Uuid::new_v4();

    let mut create_data = Data::new();
    create_data.insert("kind".into(), json!("sticky_note"));
    create_data.insert("x".into(), json!(120.0));
    create_data.insert("y".into(), json!(180.0));
    create_data.insert("props".into(), json!({ "text": "from user a", "color": "#FFEB3B" }));
    let create_req = request_bytes(board_id, "object:create", create_data);

    let a_reply =
        process_inbound_bytes(&state, &mut current_board_a, client_a_id, user_a, &client_a_tx, &create_req).await;

    assert_eq!(a_reply.len(), 1);
    assert_eq!(a_reply[0].status, Status::Done);
    assert_eq!(a_reply[0].syscall, "object:create");
    let user_a_str = user_a.to_string();
    let created_id = a_reply[0]
        .data
        .get("id")
        .and_then(|v| v.as_str())
        .expect("sender reply should include object id")
        .to_string();
    assert_eq!(
        a_reply[0].data.get("created_by").and_then(|v| v.as_str()),
        Some(user_a_str.as_str())
    );

    let b_broadcast = recv_board_broadcast(&mut client_b_rx).await;
    assert_eq!(b_broadcast.syscall, "object:create");
    assert_eq!(b_broadcast.data.get("id").and_then(|v| v.as_str()), Some(created_id.as_str()));
    assert_eq!(
        b_broadcast.data.get("created_by").and_then(|v| v.as_str()),
        Some(user_a_str.as_str())
    );
    assert_eq!(
        b_broadcast
            .data
            .get("props")
            .and_then(|v| v.get("text"))
            .and_then(|v| v.as_str()),
        Some("from user a")
    );
    // Object mutation broadcasts for direct object requests are peer-only.
    assert_no_board_broadcast(&mut client_a_rx).await;

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    assert_eq!(board.objects.len(), 1);
}

#[tokio::test]
async fn board_part_broadcasts_immediately_to_peers() {
    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board(&state).await;
    let (client_a_id, client_a_tx, mut client_a_rx, _client_b_id, _client_b_tx, mut client_b_rx) =
        register_two_clients(&state, board_id).await;
    let user_a = Uuid::new_v4();
    let mut current_board_a = Some(board_id);

    let req = request_bytes(board_id, "board:part", Data::new());
    let a_reply = process_inbound_bytes(&state, &mut current_board_a, client_a_id, user_a, &client_a_tx, &req).await;

    assert_eq!(a_reply.len(), 1);
    assert_eq!(a_reply[0].syscall, "board:part");
    assert_eq!(a_reply[0].status, Status::Done);
    assert!(current_board_a.is_none());

    let peer_broadcast = recv_board_broadcast(&mut client_b_rx).await;
    assert_eq!(peer_broadcast.syscall, "board:part");
    assert_eq!(peer_broadcast.status, Status::Request);
    let client_a_id_str = client_a_id.to_string();
    assert_eq!(
        peer_broadcast
            .data
            .get("client_id")
            .and_then(|v| v.as_str()),
        Some(client_a_id_str.as_str())
    );

    assert_no_board_broadcast(&mut client_a_rx).await;
}

#[tokio::test]
async fn multi_user_concurrent_changes_on_different_objects_sync_both_users() {
    let mut obj_a = test_helpers::dummy_object();
    obj_a.version = 1;
    obj_a.props = json!({ "text": "object a" });
    let obj_a_id = obj_a.id;

    let mut obj_b = test_helpers::dummy_object();
    obj_b.version = 1;
    obj_b.props = json!({ "text": "object b" });
    let obj_b_id = obj_b.id;

    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board_with_objects(&state, vec![obj_a, obj_b]).await;
    let (client_a_id, client_a_tx, mut client_a_rx, client_b_id, client_b_tx, mut client_b_rx) =
        register_two_clients(&state, board_id).await;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let mut current_board_a = Some(board_id);
    let mut current_board_b = Some(board_id);

    let mut update_a = Data::new();
    update_a.insert("id".into(), json!(obj_a_id));
    update_a.insert("version".into(), json!(1));
    update_a.insert("x".into(), json!(500.0));

    let mut update_b = Data::new();
    update_b.insert("id".into(), json!(obj_b_id));
    update_b.insert("version".into(), json!(1));
    update_b.insert("y".into(), json!(900.0));

    let req_a = request_bytes(board_id, "object:update", update_a);
    let req_b = request_bytes(board_id, "object:update", update_b);

    let (a_reply, b_reply) = tokio::join!(
        process_inbound_bytes(&state, &mut current_board_a, client_a_id, user_a, &client_a_tx, &req_a),
        process_inbound_bytes(&state, &mut current_board_b, client_b_id, user_b, &client_b_tx, &req_b)
    );

    assert_eq!(a_reply.len(), 1);
    assert_eq!(b_reply.len(), 1);
    assert_eq!(a_reply[0].status, Status::Done);
    assert_eq!(b_reply[0].status, Status::Done);

    // Each user receives the other user's broadcast mutation.
    let a_seen = recv_board_broadcast(&mut client_a_rx).await;
    let b_seen = recv_board_broadcast(&mut client_b_rx).await;
    assert_eq!(a_seen.syscall, "object:update");
    assert_eq!(b_seen.syscall, "object:update");
    let obj_a_id_str = obj_a_id.to_string();
    let obj_b_id_str = obj_b_id.to_string();
    assert_eq!(a_seen.data.get("id").and_then(|v| v.as_str()), Some(obj_b_id_str.as_str()));
    assert_eq!(b_seen.data.get("id").and_then(|v| v.as_str()), Some(obj_a_id_str.as_str()));

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    assert_eq!(
        board
            .objects
            .get(&obj_a_id)
            .expect("object a should exist")
            .x,
        500.0
    );
    assert_eq!(
        board
            .objects
            .get(&obj_b_id)
            .expect("object b should exist")
            .y,
        900.0
    );
}

#[tokio::test]
async fn multi_user_conflicting_same_object_edits_converge_after_retry() {
    let mut shared = test_helpers::dummy_object();
    shared.version = 1;
    shared.x = 100.0;
    shared.y = 100.0;
    let shared_id = shared.id;

    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board_with_objects(&state, vec![shared]).await;
    let (client_a_id, client_a_tx, mut client_a_rx, client_b_id, client_b_tx, mut client_b_rx) =
        register_two_clients(&state, board_id).await;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let mut current_board_a = Some(board_id);
    let mut current_board_b = Some(board_id);

    let mut update_a = Data::new();
    update_a.insert("id".into(), json!(shared_id));
    update_a.insert("version".into(), json!(1));
    update_a.insert("x".into(), json!(210.0));

    let mut update_b = Data::new();
    update_b.insert("id".into(), json!(shared_id));
    update_b.insert("version".into(), json!(1));
    update_b.insert("x".into(), json!(330.0));

    let req_a = request_bytes(board_id, "object:update", update_a);
    let req_b = request_bytes(board_id, "object:update", update_b);

    let (a_reply, b_reply) = tokio::join!(
        process_inbound_bytes(&state, &mut current_board_a, client_a_id, user_a, &client_a_tx, &req_a),
        process_inbound_bytes(&state, &mut current_board_b, client_b_id, user_b, &client_b_tx, &req_b)
    );

    assert_eq!(a_reply.len(), 1);
    assert_eq!(b_reply.len(), 1);
    let a_status = a_reply[0].status;
    let b_status = b_reply[0].status;
    assert!(
        (a_status == Status::Done && b_status == Status::Error)
            || (a_status == Status::Error && b_status == Status::Done),
        "expected one winner and one stale loser: a={a_status:?}, b={b_status:?}"
    );

    if a_status == Status::Error {
        assert_eq!(a_reply[0].data.get("code").and_then(|v| v.as_str()), Some("E_STALE_UPDATE"));
    }
    if b_status == Status::Error {
        assert_eq!(b_reply[0].data.get("code").and_then(|v| v.as_str()), Some("E_STALE_UPDATE"));
    }

    if a_status == Status::Done {
        // A won; B should receive A's winner broadcast.
        let b_seen_winner = recv_board_broadcast(&mut client_b_rx).await;
        assert_eq!(b_seen_winner.syscall, "object:update");
        assert_eq!(b_seen_winner.data.get("x").and_then(|v| v.as_f64()), Some(210.0));
        assert_no_board_broadcast(&mut client_a_rx).await;

        // B retries with the latest version to converge both clients.
        let current_version = {
            let boards = state.boards.read().await;
            boards
                .get(&board_id)
                .and_then(|b| b.objects.get(&shared_id))
                .map(|o| o.version)
                .expect("shared object should exist")
        };
        let mut retry = Data::new();
        retry.insert("id".into(), json!(shared_id));
        retry.insert("version".into(), json!(current_version));
        retry.insert("x".into(), json!(777.0));
        let retry_req = request_bytes(board_id, "object:update", retry);
        let b_retry =
            process_inbound_bytes(&state, &mut current_board_b, client_b_id, user_b, &client_b_tx, &retry_req).await;

        assert_eq!(b_retry.len(), 1);
        assert_eq!(b_retry[0].status, Status::Done);
        assert_eq!(b_retry[0].data.get("x").and_then(|v| v.as_f64()), Some(777.0));
        let a_seen_retry = recv_board_broadcast(&mut client_a_rx).await;
        assert_eq!(a_seen_retry.syscall, "object:update");
        assert_eq!(a_seen_retry.data.get("x").and_then(|v| v.as_f64()), Some(777.0));
    } else {
        // B won; A should receive B's winner broadcast.
        let a_seen_winner = recv_board_broadcast(&mut client_a_rx).await;
        assert_eq!(a_seen_winner.syscall, "object:update");
        assert_eq!(a_seen_winner.data.get("x").and_then(|v| v.as_f64()), Some(330.0));
        assert_no_board_broadcast(&mut client_b_rx).await;

        // A retries with the latest version to converge both clients.
        let current_version = {
            let boards = state.boards.read().await;
            boards
                .get(&board_id)
                .and_then(|b| b.objects.get(&shared_id))
                .map(|o| o.version)
                .expect("shared object should exist")
        };
        let mut retry = Data::new();
        retry.insert("id".into(), json!(shared_id));
        retry.insert("version".into(), json!(current_version));
        retry.insert("x".into(), json!(777.0));
        let retry_req = request_bytes(board_id, "object:update", retry);
        let a_retry =
            process_inbound_bytes(&state, &mut current_board_a, client_a_id, user_a, &client_a_tx, &retry_req).await;

        assert_eq!(a_retry.len(), 1);
        assert_eq!(a_retry[0].status, Status::Done);
        assert_eq!(a_retry[0].data.get("x").and_then(|v| v.as_f64()), Some(777.0));
        let b_seen_retry = recv_board_broadcast(&mut client_b_rx).await;
        assert_eq!(b_seen_retry.syscall, "object:update");
        assert_eq!(b_seen_retry.data.get("x").and_then(|v| v.as_f64()), Some(777.0));
    }

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    let shared_after = board
        .objects
        .get(&shared_id)
        .expect("shared object should exist");
    assert_eq!(shared_after.x, 777.0);
    assert_eq!(shared_after.version, 3);
}

#[tokio::test]
async fn multi_user_stale_update_is_rejected_and_not_broadcast() {
    let mut obj = test_helpers::dummy_object();
    obj.version = 3;
    obj.x = 90.0;
    let obj_id = obj.id;

    let state = test_helpers::test_app_state();
    let board_id = test_helpers::seed_board_with_objects(&state, vec![obj]).await;
    let (client_a_id, client_a_tx, mut client_a_rx, _client_b_id, _client_b_tx, mut client_b_rx) =
        register_two_clients(&state, board_id).await;
    let user_a = Uuid::new_v4();
    let mut current_board_a = Some(board_id);

    let mut stale_update = Data::new();
    stale_update.insert("id".into(), json!(obj_id));
    stale_update.insert("version".into(), json!(1));
    stale_update.insert("x".into(), json!(999.0));
    let stale_req = request_bytes(board_id, "object:update", stale_update);

    let a_reply =
        process_inbound_bytes(&state, &mut current_board_a, client_a_id, user_a, &client_a_tx, &stale_req).await;

    assert_eq!(a_reply.len(), 1);
    assert_eq!(a_reply[0].status, Status::Error);
    assert_eq!(a_reply[0].data.get("code").and_then(|v| v.as_str()), Some("E_STALE_UPDATE"));
    assert_no_board_broadcast(&mut client_a_rx).await;
    assert_no_board_broadcast(&mut client_b_rx).await;

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    let obj_after = board.objects.get(&obj_id).expect("object should exist");
    assert_eq!(obj_after.x, 90.0);
    assert_eq!(obj_after.version, 3);
}

#[tokio::test]
async fn ai_prompt_create_sticky_broadcasts_mutation_and_replies_with_text() {
    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".into(),
                name: "createStickyNote".into(),
                input: json!({ "text": "hello from ai", "x": 220, "y": 180, "color": "#FFEB3B" }),
            }],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 25,
            output_tokens: 30,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Created a sticky.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 10,
            output_tokens: 6,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board(&state).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let sender_frames = process_inbound_bytes(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_bytes(board_id, "create a sticky"),
    )
    .await;

    assert_eq!(sender_frames.len(), 1);
    let reply = &sender_frames[0];
    assert_eq!(reply.syscall, "ai:prompt");
    assert_eq!(reply.status, Status::Done);
    assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(reply.data.get("text").and_then(|v| v.as_str()), Some("Created a sticky."));

    let sender_broadcast = recv_board_broadcast(&mut sender_rx).await;
    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(sender_broadcast.syscall, "object:create");
    assert_eq!(peer_broadcast.syscall, "object:create");
    assert_eq!(sender_broadcast.status, Status::Done);
    assert_eq!(peer_broadcast.status, Status::Done);
    assert_eq!(sender_broadcast.data.get("kind").and_then(|v| v.as_str()), Some("sticky_note"));
    assert_eq!(peer_broadcast.data.get("kind").and_then(|v| v.as_str()), Some("sticky_note"));
    assert!(
        sender_broadcast
            .data
            .get("created_by")
            .is_some_and(serde_json::Value::is_null)
    );
    assert!(
        peer_broadcast
            .data
            .get("created_by")
            .is_some_and(serde_json::Value::is_null)
    );

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should be present");
    assert_eq!(board.objects.len(), 1);
    let created = board
        .objects
        .values()
        .next()
        .expect("created sticky should exist");
    assert_eq!(created.kind, "sticky_note");
    assert_eq!(created.props.get("text").and_then(|v| v.as_str()), Some("hello from ai"));
}

#[tokio::test]
async fn ai_prompt_resize_sticky_broadcasts_update_and_replies_with_text() {
    let mut sticky = test_helpers::dummy_object();
    sticky.version = 0;
    sticky.width = Some(120.0);
    sticky.height = Some(90.0);
    sticky.props = json!({ "text": "sticky 4", "color": "#FFEB3B" });
    let target_id = sticky.id;

    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".into(),
                name: "resizeObject".into(),
                input: json!({ "objectId": target_id, "width": 420, "height": 260 }),
            }],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 20,
            output_tokens: 28,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Resized sticky 4.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 8,
            output_tokens: 5,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board_with_objects(&state, vec![sticky]).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let sender_frames = process_inbound_bytes(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_bytes(board_id, "resize sticky 4"),
    )
    .await;

    assert_eq!(sender_frames.len(), 1);
    let reply = &sender_frames[0];
    assert_eq!(reply.syscall, "ai:prompt");
    assert_eq!(reply.status, Status::Done);
    assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(reply.data.get("text").and_then(|v| v.as_str()), Some("Resized sticky 4."));

    let sender_broadcast = recv_board_broadcast(&mut sender_rx).await;
    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(sender_broadcast.syscall, "object:update");
    assert_eq!(peer_broadcast.syscall, "object:update");
    assert_eq!(sender_broadcast.status, Status::Done);
    assert_eq!(peer_broadcast.status, Status::Done);
    let target_id_str = target_id.to_string();
    assert_eq!(
        sender_broadcast.data.get("id").and_then(|v| v.as_str()),
        Some(target_id_str.as_str())
    );
    assert_eq!(
        peer_broadcast.data.get("id").and_then(|v| v.as_str()),
        Some(target_id_str.as_str())
    );
    assert_eq!(sender_broadcast.data.get("width").and_then(|v| v.as_f64()), Some(420.0));
    assert_eq!(sender_broadcast.data.get("height").and_then(|v| v.as_f64()), Some(260.0));

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    let updated = board
        .objects
        .get(&target_id)
        .expect("updated sticky should remain on board");
    assert_eq!(updated.width, Some(420.0));
    assert_eq!(updated.height, Some(260.0));
    assert_eq!(updated.version, 1);
}

#[tokio::test]
async fn ai_prompt_multi_tool_single_turn_broadcasts_all_mutations_and_replies_with_text() {
    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_1".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "first sticky", "x": 120, "y": 140, "color": "#FFEB3B" }),
                },
                ContentBlock::ToolUse {
                    id: "tool_2".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "second sticky", "x": 360, "y": 140, "color": "#8BC34A" }),
                },
            ],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 30,
            output_tokens: 40,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Added two stickies.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 10,
            output_tokens: 8,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board(&state).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let sender_frames = process_inbound_bytes(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_bytes(board_id, "create two stickies"),
    )
    .await;

    assert_eq!(sender_frames.len(), 1);
    let reply = &sender_frames[0];
    assert_eq!(reply.syscall, "ai:prompt");
    assert_eq!(reply.status, Status::Done);
    assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(reply.data.get("text").and_then(|v| v.as_str()), Some("Added two stickies."));

    let sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
    let peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
    assert!(
        sender_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create" && f.status == Status::Done)
    );
    assert!(
        peer_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create" && f.status == Status::Done)
    );
    assert!(
        sender_broadcasts
            .iter()
            .all(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("sticky_note") })
    );
    assert!(
        peer_broadcasts
            .iter()
            .all(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("sticky_note") })
    );

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    assert_eq!(board.objects.len(), 2);
    let texts: Vec<&str> = board
        .objects
        .values()
        .filter_map(|o| o.props.get("text").and_then(|v| v.as_str()))
        .collect();
    assert!(texts.contains(&"first sticky"));
    assert!(texts.contains(&"second sticky"));
}

#[tokio::test]
async fn ai_prompt_sequence_multi_tool_text_then_multi_tool_text() {
    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_1".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "batch 1 - a", "x": 100, "y": 120, "color": "#FFEB3B" }),
                },
                ContentBlock::ToolUse {
                    id: "tool_2".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "batch 1 - b", "x": 300, "y": 120, "color": "#FFC107" }),
                },
            ],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 28,
            output_tokens: 32,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "First batch complete.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 9,
            output_tokens: 7,
        },
        ChatResponse {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_3".into(),
                    name: "createFrame".into(),
                    input: json!({ "title": "Batch 2 Frame", "x": 80, "y": 280, "width": 500, "height": 260 }),
                },
                ContentBlock::ToolUse {
                    id: "tool_4".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "batch 2 - note", "x": 140, "y": 340, "color": "#4CAF50" }),
                },
            ],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 30,
            output_tokens: 34,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Second batch complete.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 9,
            output_tokens: 7,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board(&state).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, _peer_tx, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let first_reply = process_inbound_bytes(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_bytes(board_id, "do first batch"),
    )
    .await;

    assert_eq!(first_reply.len(), 1);
    let first = &first_reply[0];
    assert_eq!(first.status, Status::Done);
    assert_eq!(first.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(first.data.get("text").and_then(|v| v.as_str()), Some("First batch complete."));
    let first_sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
    let first_peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
    assert!(
        first_sender_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create" && f.status == Status::Done)
    );
    assert!(
        first_peer_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create" && f.status == Status::Done)
    );

    let second_reply = process_inbound_bytes(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_bytes(board_id, "do second batch"),
    )
    .await;

    assert_eq!(second_reply.len(), 1);
    let second = &second_reply[0];
    assert_eq!(second.status, Status::Done);
    assert_eq!(second.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(second.data.get("text").and_then(|v| v.as_str()), Some("Second batch complete."));
    let second_sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
    let second_peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
    assert!(
        second_sender_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create" && f.status == Status::Done)
    );
    assert!(
        second_peer_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create" && f.status == Status::Done)
    );
    assert!(
        second_sender_broadcasts
            .iter()
            .any(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("frame") })
    );
    assert!(
        second_sender_broadcasts
            .iter()
            .any(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("sticky_note") })
    );

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    assert_eq!(board.objects.len(), 4);
}
