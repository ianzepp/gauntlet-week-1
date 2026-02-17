//! WebSocket handler — bidirectional frame relay.
//!
//! DESIGN
//! ======
//! On upgrade, generates a client ID and enters a `select!` loop:
//! - Incoming client frames → parse + dispatch by syscall prefix
//! - Broadcast frames from board peers → forward to client
//!
//! Handler functions are pure business logic — they validate, mutate state,
//! and return an `Outcome`. The dispatch layer owns all outbound concerns:
//! persistence, reply to sender, and broadcast to peers.
//!
//! LIFECYCLE
//! =========
//! 1. Upgrade → send `session:connected` with `client_id`
//! 2. Client sends frames → dispatch → handler returns Outcome
//! 3. Dispatch applies Outcome (reply / broadcast / both)
//! 4. Close → broadcast `board:part` → cleanup

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::frame::{Data, Frame};
use crate::services;
use crate::state::AppState;

// =============================================================================
// OUTCOME
// =============================================================================

/// Result returned by handler functions. The dispatch layer uses this to
/// decide who receives what — handlers never send frames directly.
enum Outcome {
    /// Broadcast done+data to ALL board clients including sender.
    /// Sender's copy carries `parent_id` for correlation.
    Broadcast(Data),
    /// Broadcast data to all board peers EXCLUDING sender. No reply to sender.
    /// Used for cursor moves (ephemeral, no persistence).
    BroadcastExcludeSender(Data),
    /// Send done+data to sender only.
    Reply(Data),
    /// Send empty done to sender only.
    Done,
    /// Reply to sender with one payload, broadcast different data to peers.
    ReplyAndBroadcast { reply: Data, broadcast: Data },
}

// =============================================================================
// UPGRADE
// =============================================================================

pub async fn handle_ws(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Response {
    let Some(ticket) = params.get("ticket") else {
        return (StatusCode::UNAUTHORIZED, "ticket required").into_response();
    };

    let user_id = match services::session::consume_ws_ticket(&state.pool, ticket).await {
        Ok(Some(uid)) => uid,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "invalid or expired ticket").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "ws ticket validation failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "ticket validation error").into_response();
        }
    };

    ws.on_upgrade(move |socket| run_ws(socket, state, user_id))
}

// =============================================================================
// CONNECTION
// =============================================================================

async fn run_ws(mut socket: WebSocket, state: AppState, user_id: Uuid) {
    let client_id = Uuid::new_v4();

    // Per-connection channel for receiving broadcast frames from peers.
    let (client_tx, mut client_rx) = mpsc::channel::<Frame>(256);

    // Send session:connected with user_id.
    let welcome = Frame::request("session:connected", Data::new())
        .with_data("client_id", client_id.to_string())
        .with_data("user_id", user_id.to_string());
    if send_frame(&mut socket, &state, &welcome).await.is_err() {
        return;
    }

    info!(%client_id, %user_id, "ws: client connected");

    // Track which board this client has joined.
    let mut current_board: Option<Uuid> = None;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(msg) = msg else { break };
                let Ok(msg) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        dispatch_frame(&state, &mut socket, &mut current_board, client_id, user_id, &client_tx, &text).await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Some(frame) = client_rx.recv() => {
                if send_frame(&mut socket, &state, &frame).await.is_err() {
                    break;
                }
            }
        }
    }

    // Broadcast board:part to peers BEFORE cleanup (part_board may evict state).
    if let Some(board_id) = current_board {
        let mut part_data = Data::new();
        part_data.insert("client_id".into(), serde_json::json!(client_id));
        part_data.insert("user_id".into(), serde_json::json!(user_id));
        let part_frame = Frame::request("board:part", part_data).with_board_id(board_id);
        services::board::broadcast(&state, board_id, &part_frame, Some(client_id)).await;

        services::board::part_board(&state, board_id, client_id).await;
    }
    info!(%client_id, "ws: client disconnected");
}

// =============================================================================
// FRAME DISPATCH
// =============================================================================

/// Parse an incoming JSON frame, dispatch to handler, apply outcome.
async fn dispatch_frame(
    state: &AppState,
    socket: &mut WebSocket,
    current_board: &mut Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    client_tx: &mpsc::Sender<Frame>,
    text: &str,
) {
    let sender_frames = process_inbound_text(state, current_board, client_id, user_id, client_tx, text).await;
    for frame in sender_frames {
        let _ = send_frame(socket, state, &frame).await;
    }
}

/// Parse and process one inbound text frame and return frames for the sender.
///
/// This keeps the websocket transport concerns separate from frame handling,
/// so tests can exercise frame dispatch and AI broadcast behavior end-to-end.
async fn process_inbound_text(
    state: &AppState,
    current_board: &mut Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    client_tx: &mpsc::Sender<Frame>,
    text: &str,
) -> Vec<Frame> {
    let mut req: Frame = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(e) => {
            warn!(%client_id, error = %e, "ws: invalid inbound frame");
            let err = Frame::request("gateway:error", Data::new()).with_data("message", format!("invalid json: {e}"));
            return vec![err];
        }
    };

    // Stamp the authenticated user_id as `from`.
    req.from = Some(user_id.to_string());

    let prefix = req.prefix();
    let is_cursor = prefix == "cursor";

    // Persist inbound request (skip cursors).
    if !is_cursor {
        info!(%client_id, id = %req.id, syscall = %req.syscall, status = ?req.status, "ws: recv frame");
        persist_fire_and_forget(&state.pool, &req);
    }

    // Dispatch to handler — returns Outcome or error Frame.
    let result = match prefix {
        "board" => handle_board(state, current_board, client_id, user_id, client_tx, &req).await,
        "object" => handle_object(state, *current_board, user_id, &req).await,
        "cursor" => Ok(handle_cursor(*current_board, client_id, &req)),
        "ai" => handle_ai(state, *current_board, client_id, &req).await,
        _ => Err(req.error(format!("unknown prefix: {prefix}"))),
    };

    // Apply outcome — the dispatch layer owns all outbound logic.
    let board_id = *current_board;
    match result {
        Ok(Outcome::Broadcast(data)) => {
            let sender_frame = req.done_with(data);
            // Peers get a copy without parent_id (they didn't originate the request).
            let mut peer_frame = sender_frame.clone();
            peer_frame.id = Uuid::new_v4();
            peer_frame.parent_id = None;
            if let Some(bid) = board_id {
                services::board::broadcast(state, bid, &peer_frame, Some(client_id)).await;
            }
            vec![sender_frame]
        }
        Ok(Outcome::BroadcastExcludeSender(data)) => {
            if let Some(bid) = board_id {
                let frame = Frame::request(&req.syscall, data).with_board_id(bid);
                services::board::broadcast(state, bid, &frame, Some(client_id)).await;
            }
            vec![]
        }
        Ok(Outcome::Reply(data)) => {
            vec![req.done_with(data)]
        }
        Ok(Outcome::Done) => {
            vec![req.done()]
        }
        Ok(Outcome::ReplyAndBroadcast { reply, broadcast }) => {
            let sender_frame = req.done_with(reply);
            if let Some(bid) = board_id {
                let notif = Frame::request(&req.syscall, broadcast).with_board_id(bid);
                services::board::broadcast(state, bid, &notif, Some(client_id)).await;
            }
            vec![sender_frame]
        }
        Err(err_frame) => {
            vec![err_frame]
        }
    }
}

// =============================================================================
// BOARD HANDLERS
// =============================================================================

async fn handle_board(
    state: &AppState,
    current_board: &mut Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    client_tx: &mpsc::Sender<Frame>,
    req: &Frame,
) -> Result<Outcome, Frame> {
    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);

    match op {
        "join" => {
            let Some(board_id) = req.board_id.or_else(|| {
                req.data
                    .get("board_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            }) else {
                return Err(req.error("board_id required"));
            };

            // Part current board if already joined.
            if let Some(old_board) = current_board.take() {
                services::board::part_board(state, old_board, client_id).await;
            }

            match services::board::join_board(state, board_id, client_id, client_tx.clone()).await {
                Ok(objects) => {
                    *current_board = Some(board_id);

                    let mut reply = Data::new();
                    reply.insert("objects".into(), serde_json::to_value(&objects).unwrap_or_default());

                    let mut broadcast = Data::new();
                    broadcast.insert("client_id".into(), serde_json::json!(client_id));
                    broadcast.insert("user_id".into(), serde_json::json!(user_id));

                    Ok(Outcome::ReplyAndBroadcast { reply, broadcast })
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "create" => {
            let name = req
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Board");
            match services::board::create_board(&state.pool, name).await {
                Ok(row) => {
                    let mut data = Data::new();
                    data.insert("id".into(), serde_json::json!(row.id));
                    data.insert("name".into(), serde_json::json!(row.name));
                    Ok(Outcome::Reply(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "list" => match services::board::list_boards(&state.pool).await {
            Ok(boards) => {
                let list: Vec<serde_json::Value> = boards
                    .iter()
                    .map(|b| serde_json::json!({"id": b.id, "name": b.name}))
                    .collect();
                let mut data = Data::new();
                data.insert("boards".into(), serde_json::json!(list));
                Ok(Outcome::Reply(data))
            }
            Err(e) => Err(req.error_from(&e)),
        },
        "delete" => {
            let Some(board_id) = req
                .data
                .get("board_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                return Err(req.error("board_id required"));
            };
            match services::board::delete_board(&state.pool, board_id).await {
                Ok(()) => Ok(Outcome::Done),
                Err(e) => Err(req.error_from(&e)),
            }
        }
        _ => Err(req.error(format!("unknown board op: {op}"))),
    }
}

// =============================================================================
// OBJECT HANDLERS
// =============================================================================

async fn handle_object(
    state: &AppState,
    current_board: Option<Uuid>,
    user_id: Uuid,
    req: &Frame,
) -> Result<Outcome, Frame> {
    let Some(board_id) = current_board else {
        return Err(req.error("must join a board first"));
    };

    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);

    match op {
        "create" => {
            let kind = req
                .data
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("sticky_note");
            let x = req
                .data
                .get("x")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let y = req
                .data
                .get("y")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let props = req
                .data
                .get("props")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            match services::object::create_object(state, board_id, kind, x, y, props, Some(user_id)).await {
                Ok(obj) => Ok(Outcome::Broadcast(object_to_data(&obj))),
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "update" => {
            let Some(object_id) = req
                .data
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                return Err(req.error("id required"));
            };
            let version = req
                .data
                .get("version")
                .and_then(serde_json::Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(0);

            match services::object::update_object(state, board_id, object_id, &req.data, version).await {
                Ok(obj) => Ok(Outcome::Broadcast(object_to_data(&obj))),
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "delete" => {
            let Some(object_id) = req
                .data
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                return Err(req.error("id required"));
            };

            match services::object::delete_object(state, board_id, object_id).await {
                Ok(()) => {
                    let mut data = Data::new();
                    data.insert("id".into(), serde_json::json!(object_id));
                    Ok(Outcome::Broadcast(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        _ => Err(req.error(format!("unknown object op: {op}"))),
    }
}

// =============================================================================
// CURSOR HANDLER
// =============================================================================

fn handle_cursor(current_board: Option<Uuid>, client_id: Uuid, req: &Frame) -> Outcome {
    if current_board.is_none() {
        // Silently ignore cursor moves before joining.
        return Outcome::Done;
    }

    let x = req
        .data
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = req
        .data
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let name = req
        .data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous");

    let mut data = Data::new();
    data.insert("client_id".into(), serde_json::json!(client_id));
    data.insert("x".into(), serde_json::json!(x));
    data.insert("y".into(), serde_json::json!(y));
    data.insert("name".into(), serde_json::json!(name));

    Outcome::BroadcastExcludeSender(data)
}

// =============================================================================
// AI HANDLER (exception: broadcasts mutations directly)
// =============================================================================

async fn handle_ai(
    state: &AppState,
    current_board: Option<Uuid>,
    client_id: Uuid,
    req: &Frame,
) -> Result<Outcome, Frame> {
    let Some(board_id) = current_board else {
        return Err(req.error("must join a board first"));
    };

    let Some(llm) = &state.llm else {
        return Err(req.error("AI features not configured"));
    };

    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);
    match op {
        "prompt" => {
            let prompt = req
                .data
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if prompt.is_empty() {
                return Err(req.error("prompt required"));
            }

            match services::ai::handle_prompt(state, llm, board_id, client_id, prompt).await {
                Ok(result) => {
                    // AI is the one exception: broadcast mutations directly.
                    for mutation in &result.mutations {
                        let (syscall, data) = match mutation {
                            services::ai::AiMutation::Created(obj) => ("object:create", object_to_data(obj)),
                            services::ai::AiMutation::Updated(obj) => ("object:update", object_to_data(obj)),
                            services::ai::AiMutation::Deleted(id) => {
                                let mut d = Data::new();
                                d.insert("id".into(), serde_json::json!(id));
                                ("object:delete", d)
                            }
                        };
                        let frame = Frame::request(syscall, data).with_board_id(board_id);
                        services::board::broadcast(state, board_id, &frame, None).await;
                    }

                    let mut data = Data::new();
                    if let Some(text) = &result.text {
                        data.insert("text".into(), serde_json::json!(text));
                    }
                    data.insert("mutations".into(), serde_json::json!(result.mutations.len()));
                    Ok(Outcome::Reply(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        _ => Err(req.error(format!("unknown ai op: {op}"))),
    }
}

// =============================================================================
// HELPERS
// =============================================================================

async fn send_frame(socket: &mut WebSocket, state: &AppState, frame: &Frame) -> Result<(), ()> {
    let json = match serde_json::to_string(frame) {
        Ok(j) => j,
        Err(e) => {
            warn!(error = %e, "ws: failed to serialize frame");
            return Err(());
        }
    };
    let is_cursor = frame.syscall.starts_with("cursor:");
    if !is_cursor {
        if frame.status == crate::frame::Status::Error {
            let code = frame
                .data
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let message = frame
                .data
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            warn!(id = %frame.id, syscall = %frame.syscall, code, message, "ws: send frame status=Error");
        } else {
            info!(id = %frame.id, syscall = %frame.syscall, status = ?frame.status, "ws: send frame");
        }
    }
    let result = socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ());
    if result.is_ok() && !is_cursor {
        persist_fire_and_forget(&state.pool, frame);
    }
    result
}

/// Spawn a fire-and-forget task to persist a frame to the database.
fn persist_fire_and_forget(pool: &sqlx::PgPool, frame: &Frame) {
    let pool = pool.clone();
    let frame = frame.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::services::persistence::persist_frame(&pool, &frame).await {
            warn!(error = %e, "frame persist failed");
        }
    });
}

fn object_to_data(obj: &crate::state::BoardObject) -> Data {
    let mut data = Data::new();
    data.insert("id".into(), serde_json::json!(obj.id));
    data.insert("board_id".into(), serde_json::json!(obj.board_id));
    data.insert("kind".into(), serde_json::json!(obj.kind));
    data.insert("x".into(), serde_json::json!(obj.x));
    data.insert("y".into(), serde_json::json!(obj.y));
    data.insert("width".into(), serde_json::json!(obj.width));
    data.insert("height".into(), serde_json::json!(obj.height));
    data.insert("rotation".into(), serde_json::json!(obj.rotation));
    data.insert("z_index".into(), serde_json::json!(obj.z_index));
    data.insert("props".into(), obj.props.clone());
    data.insert("version".into(), serde_json::json!(obj.version));
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Status;
    use crate::llm::types::{ChatResponse, ContentBlock, LlmChat, LlmError, Message, Tool};
    use crate::state::test_helpers;
    use serde_json::json;
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

    fn ai_prompt_json(board_id: Uuid, prompt: &str) -> String {
        let mut data = Data::new();
        data.insert("prompt".into(), json!(prompt));
        let req = Frame::request("ai:prompt", data).with_board_id(board_id);
        serde_json::to_string(&req).expect("frame should serialize")
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

    async fn register_two_clients(
        state: &AppState,
        board_id: Uuid,
    ) -> (Uuid, mpsc::Sender<Frame>, mpsc::Receiver<Frame>, Uuid, mpsc::Receiver<Frame>) {
        let sender_client_id = Uuid::new_v4();
        let peer_client_id = Uuid::new_v4();

        let (sender_tx, sender_rx) = mpsc::channel(32);
        let (peer_tx, peer_rx) = mpsc::channel(32);

        let mut boards = state.boards.write().await;
        let board = boards
            .get_mut(&board_id)
            .expect("board should exist in memory");
        board.clients.insert(sender_client_id, sender_tx.clone());
        board.clients.insert(peer_client_id, peer_tx);

        (sender_client_id, sender_tx, sender_rx, peer_client_id, peer_rx)
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

        let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
            register_two_clients(&state, board_id).await;
        let user_id = Uuid::new_v4();
        let mut current_board = Some(board_id);

        let sender_frames = process_inbound_text(
            &state,
            &mut current_board,
            sender_client_id,
            user_id,
            &sender_tx,
            &ai_prompt_json(board_id, "create a sticky"),
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
        assert_eq!(
            sender_broadcast
                .data
                .get("kind")
                .and_then(|v| v.as_str()),
            Some("sticky_note")
        );
        assert_eq!(
            peer_broadcast
                .data
                .get("kind")
                .and_then(|v| v.as_str()),
            Some("sticky_note")
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

        let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
            register_two_clients(&state, board_id).await;
        let user_id = Uuid::new_v4();
        let mut current_board = Some(board_id);

        let sender_frames = process_inbound_text(
            &state,
            &mut current_board,
            sender_client_id,
            user_id,
            &sender_tx,
            &ai_prompt_json(board_id, "resize sticky 4"),
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
        let target_id_str = target_id.to_string();
        assert_eq!(
            sender_broadcast.data.get("id").and_then(|v| v.as_str()),
            Some(target_id_str.as_str())
        );
        assert_eq!(
            peer_broadcast.data.get("id").and_then(|v| v.as_str()),
            Some(target_id_str.as_str())
        );
        assert_eq!(
            sender_broadcast
                .data
                .get("width")
                .and_then(|v| v.as_f64()),
            Some(420.0)
        );
        assert_eq!(
            sender_broadcast
                .data
                .get("height")
                .and_then(|v| v.as_f64()),
            Some(260.0)
        );

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

        let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
            register_two_clients(&state, board_id).await;
        let user_id = Uuid::new_v4();
        let mut current_board = Some(board_id);

        let sender_frames = process_inbound_text(
            &state,
            &mut current_board,
            sender_client_id,
            user_id,
            &sender_tx,
            &ai_prompt_json(board_id, "create two stickies"),
        )
        .await;

        assert_eq!(sender_frames.len(), 1);
        let reply = &sender_frames[0];
        assert_eq!(reply.syscall, "ai:prompt");
        assert_eq!(reply.status, Status::Done);
        assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            reply.data.get("text").and_then(|v| v.as_str()),
            Some("Added two stickies.")
        );

        let sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
        let peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
        assert!(sender_broadcasts.iter().all(|f| f.syscall == "object:create"));
        assert!(peer_broadcasts.iter().all(|f| f.syscall == "object:create"));
        assert!(sender_broadcasts.iter().all(|f| {
            f.data
                .get("kind")
                .and_then(|v| v.as_str())
                == Some("sticky_note")
        }));
        assert!(peer_broadcasts.iter().all(|f| {
            f.data
                .get("kind")
                .and_then(|v| v.as_str())
                == Some("sticky_note")
        }));

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

        let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
            register_two_clients(&state, board_id).await;
        let user_id = Uuid::new_v4();
        let mut current_board = Some(board_id);

        let first_reply = process_inbound_text(
            &state,
            &mut current_board,
            sender_client_id,
            user_id,
            &sender_tx,
            &ai_prompt_json(board_id, "do first batch"),
        )
        .await;

        assert_eq!(first_reply.len(), 1);
        let first = &first_reply[0];
        assert_eq!(first.status, Status::Done);
        assert_eq!(first.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            first.data.get("text").and_then(|v| v.as_str()),
            Some("First batch complete.")
        );
        let first_sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
        let first_peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
        assert!(first_sender_broadcasts.iter().all(|f| f.syscall == "object:create"));
        assert!(first_peer_broadcasts.iter().all(|f| f.syscall == "object:create"));

        let second_reply = process_inbound_text(
            &state,
            &mut current_board,
            sender_client_id,
            user_id,
            &sender_tx,
            &ai_prompt_json(board_id, "do second batch"),
        )
        .await;

        assert_eq!(second_reply.len(), 1);
        let second = &second_reply[0];
        assert_eq!(second.status, Status::Done);
        assert_eq!(second.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            second.data.get("text").and_then(|v| v.as_str()),
            Some("Second batch complete.")
        );
        let second_sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
        let second_peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
        assert!(second_sender_broadcasts.iter().all(|f| f.syscall == "object:create"));
        assert!(second_peer_broadcasts.iter().all(|f| f.syscall == "object:create"));
        assert!(second_sender_broadcasts.iter().any(|f| {
            f.data
                .get("kind")
                .and_then(|v| v.as_str())
                == Some("frame")
        }));
        assert!(second_sender_broadcasts.iter().any(|f| {
            f.data
                .get("kind")
                .and_then(|v| v.as_str())
                == Some("sticky_note")
        }));

        let boards = state.boards.read().await;
        let board = boards.get(&board_id).expect("board should exist");
        assert_eq!(board.objects.len(), 4);
    }
}
