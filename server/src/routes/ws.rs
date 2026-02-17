//! WebSocket handler — bidirectional frame relay.
//!
//! DESIGN
//! ======
//! Ported from Prior's `gateways/api/src/ws.rs` with CollabBoard-specific
//! dispatch. On upgrade, generates an anonymous client ID and enters a
//! `select!` loop that handles:
//! - Incoming client frames → parse + dispatch by syscall prefix
//! - Broadcast frames from board peers → forward to client
//!
//! LIFECYCLE
//! =========
//! 1. Upgrade → send `session:connected` with `client_id`
//! 2. Client sends frames → validate prefix → dispatch to service
//! 3. Broadcast frames from peers → forwarded to client
//! 4. Close → `board:part` → cleanup

use std::collections::HashMap;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::frame::{Data, Frame};
use crate::services;
use crate::state::AppState;

// =============================================================================
// UPGRADE
// =============================================================================

pub async fn handle_ws(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Response {
    // Validate WS ticket.
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

    // Main loop — same select! pattern as Prior's ws.rs.
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

    // Cleanup: part the board if joined.
    if let Some(board_id) = current_board {
        services::board::part_board(&state, board_id, client_id).await;
    }
    info!(%client_id, "ws: client disconnected");
}

// =============================================================================
// FRAME DISPATCH
// =============================================================================

/// Parse an incoming JSON frame and dispatch by syscall prefix.
async fn dispatch_frame(
    state: &AppState,
    socket: &mut WebSocket,
    current_board: &mut Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    client_tx: &mpsc::Sender<Frame>,
    text: &str,
) {
    let req: WsRequest = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(e) => {
            let err = Frame::request("gateway:error", Data::new()).with_data("message", format!("invalid json: {e}"));
            let _ = send_frame(socket, state, &err).await;
            return;
        }
    };

    let prefix = req
        .syscall
        .split_once(':')
        .map_or(req.syscall.as_str(), |(p, _)| p);

    match prefix {
        "board" => handle_board(state, socket, current_board, client_id, user_id, client_tx, &req).await,
        "object" => handle_object(state, socket, *current_board, client_id, user_id, &req).await,
        "cursor" => handle_cursor(state, *current_board, client_id, user_id, &req).await,
        "ai" => handle_ai(state, socket, *current_board, client_id, user_id, &req).await,
        _ => {
            let err =
                Frame::request("gateway:error", Data::new()).with_data("message", format!("unknown prefix: {prefix}"));
            let _ = send_frame(socket, state, &err).await;
        }
    }
}

// =============================================================================
// BOARD HANDLERS
// =============================================================================

async fn handle_board(
    state: &AppState,
    socket: &mut WebSocket,
    current_board: &mut Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    client_tx: &mpsc::Sender<Frame>,
    req: &WsRequest,
) {
    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);
    let parent = Frame::request(&req.syscall, req.data.clone()).with_from(user_id.to_string());

    match op {
        "join" => {
            let Some(board_id) = req.board_id.or_else(|| {
                req.data
                    .get("board_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            }) else {
                let _ = send_frame(socket, state, &parent.error("board_id required")).await;
                return;
            };

            // Part current board if already joined.
            if let Some(old_board) = current_board.take() {
                services::board::part_board(state, old_board, client_id).await;
            }

            match services::board::join_board(state, board_id, client_id, client_tx.clone()).await {
                Ok(objects) => {
                    let mut data = Data::new();
                    data.insert("objects".into(), serde_json::to_value(&objects).unwrap_or_default());
                    let _ = send_frame(socket, state, &parent.item(data)).await;
                    let _ = send_frame(socket, state, &parent.done()).await;
                    *current_board = Some(board_id);
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
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
                    let _ = send_frame(socket, state, &parent.item(data)).await;
                    let _ = send_frame(socket, state, &parent.done()).await;
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
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
                let _ = send_frame(socket, state, &parent.item(data)).await;
                let _ = send_frame(socket, state, &parent.done()).await;
            }
            Err(e) => {
                let _ = send_frame(socket, state, &parent.error_from(&e)).await;
            }
        },
        "delete" => {
            let Some(board_id) = req
                .data
                .get("board_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                let _ = send_frame(socket, state, &parent.error("board_id required")).await;
                return;
            };
            match services::board::delete_board(&state.pool, board_id).await {
                Ok(()) => {
                    let _ = send_frame(socket, state, &parent.done()).await;
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
            }
        }
        _ => {
            let _ = send_frame(socket, state, &parent.error(format!("unknown board op: {op}"))).await;
        }
    }
}

// =============================================================================
// OBJECT HANDLERS
// =============================================================================

async fn handle_object(
    state: &AppState,
    socket: &mut WebSocket,
    current_board: Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    req: &WsRequest,
) {
    let Some(board_id) = current_board else {
        let parent = Frame::request(&req.syscall, Data::new()).with_from(user_id.to_string());
        let _ = send_frame(socket, state, &parent.error("must join a board first")).await;
        return;
    };

    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);
    let parent = Frame::request(&req.syscall, req.data.clone())
        .with_board_id(board_id)
        .with_from(user_id.to_string());

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
                Ok(obj) => {
                    let data = object_to_data(&obj);
                    let broadcast = Frame::request("object:created", data.clone()).with_board_id(board_id);
                    services::board::broadcast(state, board_id, &broadcast, None).await;
                    let _ = send_frame(socket, state, &parent.item(data)).await;
                    let _ = send_frame(socket, state, &parent.done()).await;
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
            }
        }
        "update" => {
            let Some(object_id) = req
                .data
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                let _ = send_frame(socket, state, &parent.error("id required")).await;
                return;
            };
            let version = req
                .data
                .get("version")
                .and_then(serde_json::Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(0);

            match services::object::update_object(state, board_id, object_id, &req.data, version).await {
                Ok(obj) => {
                    let data = object_to_data(&obj);
                    let broadcast = Frame::request("object:updated", data.clone()).with_board_id(board_id);
                    services::board::broadcast(state, board_id, &broadcast, Some(client_id)).await;
                    let _ = send_frame(socket, state, &parent.item(data)).await;
                    let _ = send_frame(socket, state, &parent.done()).await;
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
            }
        }
        "delete" => {
            let Some(object_id) = req
                .data
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                let _ = send_frame(socket, state, &parent.error("id required")).await;
                return;
            };

            match services::object::delete_object(state, board_id, object_id).await {
                Ok(()) => {
                    let mut data = Data::new();
                    data.insert("id".into(), serde_json::json!(object_id));
                    let broadcast = Frame::request("object:deleted", data.clone()).with_board_id(board_id);
                    services::board::broadcast(state, board_id, &broadcast, None).await;
                    let _ = send_frame(socket, state, &parent.done()).await;
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
            }
        }
        _ => {
            let _ = send_frame(socket, state, &parent.error(format!("unknown object op: {op}"))).await;
        }
    }
}

// =============================================================================
// CURSOR HANDLERS
// =============================================================================

async fn handle_cursor(
    state: &AppState,
    current_board: Option<Uuid>,
    client_id: Uuid,
    _user_id: Uuid,
    req: &WsRequest,
) {
    let Some(board_id) = current_board else {
        return; // Silently ignore cursor moves before joining.
    };

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

    services::cursor::broadcast_cursor(state, board_id, client_id, x, y, name).await;
}

// =============================================================================
// AI HANDLERS
// =============================================================================

async fn handle_ai(
    state: &AppState,
    socket: &mut WebSocket,
    current_board: Option<Uuid>,
    client_id: Uuid,
    user_id: Uuid,
    req: &WsRequest,
) {
    let Some(board_id) = current_board else {
        let parent = Frame::request(&req.syscall, Data::new()).with_from(user_id.to_string());
        let _ = send_frame(socket, state, &parent.error("must join a board first")).await;
        return;
    };

    let parent = Frame::request(&req.syscall, req.data.clone())
        .with_board_id(board_id)
        .with_from(user_id.to_string());

    let Some(llm) = &state.llm else {
        let _ = send_frame(socket, state, &parent.error("AI features not configured")).await;
        return;
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
                let _ = send_frame(socket, state, &parent.error("prompt required")).await;
                return;
            }

            match services::ai::handle_prompt(state, llm, board_id, client_id, prompt).await {
                Ok(result) => {
                    // Broadcast all object mutations to board peers.
                    for mutation in &result.mutations {
                        match mutation {
                            services::ai::AiMutation::Created(obj) => {
                                let data = object_to_data(obj);
                                let broadcast = Frame::request("object:created", data).with_board_id(board_id);
                                services::board::broadcast(state, board_id, &broadcast, None).await;
                            }
                            services::ai::AiMutation::Updated(obj) => {
                                let data = object_to_data(obj);
                                let broadcast = Frame::request("object:updated", data).with_board_id(board_id);
                                services::board::broadcast(state, board_id, &broadcast, None).await;
                            }
                            services::ai::AiMutation::Deleted(id) => {
                                let mut data = Data::new();
                                data.insert("id".into(), serde_json::json!(id));
                                let broadcast = Frame::request("object:deleted", data).with_board_id(board_id);
                                services::board::broadcast(state, board_id, &broadcast, None).await;
                            }
                        }
                    }

                    // Send text response to requesting client.
                    if let Some(text) = &result.text {
                        let mut data = Data::new();
                        data.insert("text".into(), serde_json::json!(text));
                        data.insert("mutations".into(), serde_json::json!(result.mutations.len()));
                        let _ = send_frame(socket, state, &parent.item(data)).await;
                    }
                    let _ = send_frame(socket, state, &parent.done()).await;
                }
                Err(e) => {
                    let _ = send_frame(socket, state, &parent.error_from(&e)).await;
                }
            }
        }
        _ => {
            let _ = send_frame(socket, state, &parent.error(format!("unknown ai op: {op}"))).await;
        }
    }
}

// =============================================================================
// TYPES
// =============================================================================

#[derive(serde::Deserialize)]
struct WsRequest {
    syscall: String,
    #[serde(default)]
    data: Data,
    #[serde(default)]
    board_id: Option<Uuid>,
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
    let result = socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ());
    if result.is_ok() {
        // Fire-and-forget: persist frame directly to DB.
        let pool = state.pool.clone();
        let frame = frame.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::services::persistence::persist_frame(&pool, &frame).await {
                warn!(error = %e, "ws: frame persist failed");
            }
        });
    }
    result
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
