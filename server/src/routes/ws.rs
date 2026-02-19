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

const DEFAULT_WS_CLIENT_CHANNEL_CAPACITY: usize = 256;

fn ws_client_channel_capacity() -> usize {
    std::env::var("WS_CLIENT_CHANNEL_CAPACITY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_WS_CLIENT_CHANNEL_CAPACITY)
}

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
    let (user_name, user_color) = match fetch_user_identity(&state, user_id).await {
        Ok(identity) => identity,
        Err(e) => {
            warn!(%user_id, error = %e, "ws: failed to fetch user profile");
            ("Agent".to_owned(), "#8a8178".to_owned())
        }
    };

    // Per-connection channel for receiving broadcast frames from peers.
    let (client_tx, mut client_rx) = mpsc::channel::<Frame>(ws_client_channel_capacity());

    // Send session:connected with user_id.
    let welcome = Frame::request("session:connected", Data::new())
        .with_data("client_id", client_id.to_string())
        .with_data("user_id", user_id.to_string())
        .with_data("user_name", user_name.clone())
        .with_data("user_color", user_color.clone());
    if send_frame(&mut socket, &welcome).await.is_err() {
        return;
    }
    services::persistence::enqueue_frame(&state, &welcome);

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
                        dispatch_frame(
                            &state,
                            &mut socket,
                            &mut current_board,
                            client_id,
                            user_id,
                            &user_name,
                            &user_color,
                            &client_tx,
                            &text,
                        )
                        .await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Some(frame) = client_rx.recv() => {
                if send_frame(&mut socket, &frame).await.is_err() {
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
        part_data.insert("user_name".into(), serde_json::json!(user_name));
        part_data.insert("user_color".into(), serde_json::json!(user_color));
        let part_frame = Frame::request("board:part", part_data).with_board_id(board_id);
        services::persistence::enqueue_frame(&state, &part_frame);
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
    user_name: &str,
    user_color: &str,
    client_tx: &mpsc::Sender<Frame>,
    text: &str,
) {
    let sender_frames =
        process_inbound_text(state, current_board, client_id, user_id, user_name, user_color, client_tx, text).await;
    for frame in sender_frames {
        let _ = send_frame(socket, &frame).await;
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
    user_name: &str,
    user_color: &str,
    client_tx: &mpsc::Sender<Frame>,
    text: &str,
) -> Vec<Frame> {
    let mut req: Frame = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(e) => {
            warn!(%client_id, error = %e, "ws: invalid inbound frame");
            let err = Frame::request("gateway:error", Data::new()).with_data("message", format!("invalid json: {e}"));
            services::persistence::enqueue_frame(state, &err);
            return vec![err];
        }
    };

    // Stamp the authenticated user_id as `from`.
    req.from = Some(user_id.to_string());

    let prefix = req.prefix();
    let is_ephemeral = prefix == "cursor" || req.syscall == "object:drag" || req.syscall == "object:drag:end";

    // Persist inbound request (skip ephemeral frames).
    if !is_ephemeral {
        info!(%client_id, id = %req.id, syscall = %req.syscall, status = ?req.status, "ws: recv frame");
        services::persistence::enqueue_frame(state, &req);
    }

    // Dispatch to handler — returns Outcome or error Frame.
    let result = match prefix {
        "board" => handle_board(state, current_board, client_id, user_id, user_name, user_color, client_tx, &req).await,
        "object" => handle_object(state, *current_board, user_id, &req).await,
        "chat" => handle_chat(state, *current_board, &req).await,
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
            services::persistence::enqueue_frame(state, &sender_frame);
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
            let sender_frame = req.done_with(data);
            services::persistence::enqueue_frame(state, &sender_frame);
            vec![sender_frame]
        }
        Ok(Outcome::Done) => {
            let sender_frame = req.done();
            services::persistence::enqueue_frame(state, &sender_frame);
            vec![sender_frame]
        }
        Ok(Outcome::ReplyAndBroadcast { reply, broadcast }) => {
            let sender_frame = req.done_with(reply);
            if let Some(bid) = board_id {
                let notif = Frame::request(&req.syscall, broadcast).with_board_id(bid);
                services::persistence::enqueue_frame(state, &notif);
                services::board::broadcast(state, bid, &notif, Some(client_id)).await;
            }
            services::persistence::enqueue_frame(state, &sender_frame);
            vec![sender_frame]
        }
        Err(err_frame) => {
            services::persistence::enqueue_frame(state, &err_frame);
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
    user_name: &str,
    user_color: &str,
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

            match services::board::join_board(
                state,
                board_id,
                user_id,
                user_name,
                user_color,
                client_id,
                client_tx.clone(),
            )
            .await
            {
                Ok(objects) => {
                    *current_board = Some(board_id);

                    let mut reply = Data::new();
                    reply.insert("objects".into(), serde_json::to_value(&objects).unwrap_or_default());
                    if let Ok(Some(name)) = sqlx::query_scalar::<_, String>("SELECT name FROM boards WHERE id = $1")
                        .bind(board_id)
                        .fetch_optional(&state.pool)
                        .await
                    {
                        reply.insert("name".into(), serde_json::json!(name));
                    }

                    let mut broadcast = Data::new();
                    broadcast.insert("client_id".into(), serde_json::json!(client_id));
                    broadcast.insert("user_id".into(), serde_json::json!(user_id));
                    broadcast.insert("user_name".into(), serde_json::json!(user_name));
                    broadcast.insert("user_color".into(), serde_json::json!(user_color));

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
            match services::board::create_board(&state.pool, name, user_id).await {
                Ok(row) => {
                    let mut data = Data::new();
                    data.insert("id".into(), serde_json::json!(row.id));
                    data.insert("name".into(), serde_json::json!(row.name));
                    Ok(Outcome::Reply(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "list" => match services::board::list_boards(&state.pool, user_id).await {
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
        "users:list" => {
            let board_id = req.board_id.or(*current_board).or_else(|| {
                req.data
                    .get("board_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            });
            let Some(board_id) = board_id else {
                return Err(req.error("board_id required"));
            };

            let users = services::board::list_board_users(state, board_id).await;
            let users_json: Vec<serde_json::Value> = users
                .into_iter()
                .map(|u| {
                    serde_json::json!({
                        "client_id": u.client_id,
                        "user_id": u.user_id,
                        "user_name": u.user_name,
                        "user_color": u.user_color
                    })
                })
                .collect();
            let mut data = Data::new();
            data.insert("users".into(), serde_json::json!(users_json));
            Ok(Outcome::Reply(data))
        }
        "delete" => {
            let Some(board_id) = req
                .data
                .get("board_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
            else {
                return Err(req.error("board_id required"));
            };
            match services::board::delete_board(&state.pool, board_id, user_id).await {
                Ok(()) => Ok(Outcome::Done),
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "savepoint:create" => {
            let board_id = req.board_id.or(*current_board).or_else(|| {
                req.data
                    .get("board_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            });
            let Some(board_id) = board_id else {
                return Err(req.error("board_id required"));
            };

            let label = req.data.get("label").and_then(|v| v.as_str());
            match services::savepoint::create_savepoint(state, board_id, user_id, label, false, "manual").await {
                Ok(row) => {
                    let mut data = Data::new();
                    data.insert("savepoint".into(), services::savepoint::savepoint_row_to_json(row));
                    Ok(Outcome::Reply(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "savepoint:list" => {
            let board_id = req.board_id.or(*current_board).or_else(|| {
                req.data
                    .get("board_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            });
            let Some(board_id) = board_id else {
                return Err(req.error("board_id required"));
            };

            match services::savepoint::list_savepoints(state, board_id, user_id).await {
                Ok(rows) => {
                    let mut data = Data::new();
                    data.insert(
                        "savepoints".into(),
                        serde_json::json!(services::savepoint::savepoint_rows_to_json(rows)),
                    );
                    Ok(Outcome::Reply(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        _ => Err(req.error(format!("unknown board op: {op}"))),
    }
}

// =============================================================================
// CHAT HANDLER
// =============================================================================

async fn handle_chat(state: &AppState, current_board: Option<Uuid>, req: &Frame) -> Result<Outcome, Frame> {
    let Some(board_id) = current_board else {
        return Err(req.error("must join a board first"));
    };

    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);
    match op {
        "message" => {
            let message = req
                .data
                .get("message")
                .and_then(|v| v.as_str())
                .map_or("", str::trim);

            if message.is_empty() {
                return Err(req.error("message required"));
            }

            let mut data = Data::new();
            data.insert("message".into(), serde_json::json!(message));
            Ok(Outcome::Broadcast(data))
        }
        "history" => {
            let rows = match sqlx::query_as::<_, (Uuid, i64, Option<String>, Option<String>)>(
                r#"SELECT id, ts, "from", data->>'message' AS message
                   FROM frames
                   WHERE board_id = $1 AND syscall = 'chat:message' AND status = 'request'
                   ORDER BY seq ASC
                   LIMIT 200"#,
            )
            .bind(board_id)
            .fetch_all(&state.pool)
            .await
            {
                Ok(rows) => rows,
                Err(e) => return Err(req.error(format!("chat history failed: {e}"))),
            };

            let messages: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, ts, from, message)| {
                    serde_json::json!({
                        "id": id,
                        "ts": ts,
                        "from": from,
                        "message": message.unwrap_or_default(),
                    })
                })
                .collect();

            let mut data = Data::new();
            data.insert("messages".into(), serde_json::json!(messages));
            Ok(Outcome::Reply(data))
        }
        _ => Err(req.error(format!("unknown chat op: {op}"))),
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
            let width = req.data.get("width").and_then(serde_json::Value::as_f64);
            let height = req.data.get("height").and_then(serde_json::Value::as_f64);
            let rotation = req
                .data
                .get("rotation")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let props = req
                .data
                .get("props")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            match services::object::create_object(
                state,
                board_id,
                kind,
                x,
                y,
                width,
                height,
                rotation,
                props,
                Some(user_id),
            )
            .await
            {
                Ok(obj) => {
                    if state.frame_persist_tx.is_some()
                        && let Err(e) =
                            services::savepoint::maybe_create_auto_savepoint(state, board_id, user_id, "object:create")
                                .await
                    {
                        warn!(error = %e, %board_id, object_id = %obj.id, "auto savepoint create failed");
                    }
                    Ok(Outcome::Broadcast(object_to_data(&obj)))
                }
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
                    if state.frame_persist_tx.is_some()
                        && let Err(e) =
                            services::savepoint::maybe_create_auto_savepoint(state, board_id, user_id, "object:delete")
                                .await
                    {
                        warn!(error = %e, %board_id, %object_id, "auto savepoint create failed");
                    }
                    let mut data = Data::new();
                    data.insert("id".into(), serde_json::json!(object_id));
                    Ok(Outcome::Broadcast(data))
                }
                Err(e) => Err(req.error_from(&e)),
            }
        }
        "drag" => {
            let Some(object_id) = req
                .data
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Uuid>().ok())
            else {
                return Err(req.error("id required"));
            };

            let mut data = Data::new();
            data.insert("id".into(), serde_json::json!(object_id));
            for key in ["x", "y", "width", "height", "rotation", "z_index", "props"] {
                if let Some(value) = req.data.get(key) {
                    data.insert(key.into(), value.clone());
                }
            }
            Ok(Outcome::BroadcastExcludeSender(data))
        }
        "drag:end" => {
            let Some(object_id) = req
                .data
                .get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Uuid>().ok())
            else {
                return Err(req.error("id required"));
            };
            let mut data = Data::new();
            data.insert("id".into(), serde_json::json!(object_id));
            Ok(Outcome::BroadcastExcludeSender(data))
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
        return Outcome::BroadcastExcludeSender(Data::new());
    }

    let op = req.syscall.split_once(':').map_or("moved", |(_, op)| op);
    match op {
        "clear" => {
            let mut data = Data::new();
            data.insert("client_id".into(), serde_json::json!(client_id));
            Outcome::BroadcastExcludeSender(data)
        }
        _ => {
            let mut data = Data::new();
            data.insert("client_id".into(), serde_json::json!(client_id));
            if let Some(x) = req.data.get("x").and_then(serde_json::Value::as_f64) {
                data.insert("x".into(), serde_json::json!(x));
            }
            if let Some(y) = req.data.get("y").and_then(serde_json::Value::as_f64) {
                data.insert("y".into(), serde_json::json!(y));
            }
            if let Some(center_x) = req
                .data
                .get("camera_center_x")
                .and_then(serde_json::Value::as_f64)
            {
                data.insert("camera_center_x".into(), serde_json::json!(center_x));
            }
            if let Some(center_y) = req
                .data
                .get("camera_center_y")
                .and_then(serde_json::Value::as_f64)
            {
                data.insert("camera_center_y".into(), serde_json::json!(center_y));
            }
            if let Some(zoom) = req
                .data
                .get("camera_zoom")
                .and_then(serde_json::Value::as_f64)
            {
                data.insert("camera_zoom".into(), serde_json::json!(zoom));
            }

            Outcome::BroadcastExcludeSender(data)
        }
    }
}

async fn fetch_user_identity(state: &AppState, user_id: Uuid) -> Result<(String, String), sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String)>("SELECT name, color FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;
    Ok(row)
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
    let Some(user_id) = req.from.as_deref().and_then(|s| s.parse::<Uuid>().ok()) else {
        return Err(req.error("missing authenticated user id"));
    };

    let op = req.syscall.split_once(':').map_or("", |(_, op)| op);
    match op {
        "prompt" => {
            let prompt = req
                .data
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let grid_context = req
                .data
                .get("grid_context")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);

            if prompt.is_empty() {
                return Err(req.error("prompt required"));
            }

            match services::ai::handle_prompt(state, llm, board_id, client_id, user_id, prompt, grid_context.as_deref())
                .await
            {
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
                        let mut frame = Frame::request(syscall, data)
                            .with_board_id(board_id)
                            .with_from(user_id.to_string());
                        frame.status = crate::frame::Status::Done;
                        services::persistence::enqueue_frame(state, &frame);
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
        "history" => ai_history(state, board_id, user_id, req).await,
        _ => Err(req.error(format!("unknown ai op: {op}"))),
    }
}

async fn ai_history(state: &AppState, board_id: Uuid, user_id: Uuid, req: &Frame) -> Result<Outcome, Frame> {
    let rows = match sqlx::query_as::<_, (Uuid, i64, String, Option<String>, Option<String>, Option<String>)>(
        "SELECT f.id, f.ts, f.status::text,
                f.data->>'prompt' AS prompt,
                f.data->>'text' AS text,
                f.data->>'mutations' AS mutations
         FROM frames f
         WHERE f.board_id = $1
           AND f.\"from\" = $2
           AND f.syscall = 'ai:prompt'
           AND f.status IN ('request', 'done')
         ORDER BY f.seq ASC
         LIMIT 400",
    )
    .bind(board_id)
    .bind(user_id.to_string())
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => return Err(req.error(format!("ai history failed: {e}"))),
    };

    let messages: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|(id, ts, status, prompt, text, mutations)| {
            if status == "request" {
                let prompt = prompt?;
                if prompt.is_empty() {
                    return None;
                }
                Some(serde_json::json!({
                    "id": id,
                    "ts": ts,
                    "role": "user",
                    "text": prompt,
                }))
            } else {
                let text = text?;
                if text.is_empty() {
                    return None;
                }
                let mut msg = serde_json::json!({
                    "id": id,
                    "ts": ts,
                    "role": "assistant",
                    "text": text,
                });
                if let Some(m) = mutations {
                    if let Ok(n) = m.parse::<u64>() {
                        msg["mutations"] = serde_json::json!(n);
                    }
                }
                Some(msg)
            }
        })
        .collect();

    let mut data = Data::new();
    data.insert("messages".into(), serde_json::json!(messages));
    Ok(Outcome::Reply(data))
}

// =============================================================================
// HELPERS
// =============================================================================

async fn send_frame(socket: &mut WebSocket, frame: &Frame) -> Result<(), ()> {
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
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ())
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
    data.insert("created_by".into(), serde_json::json!(obj.created_by));
    data.insert("version".into(), serde_json::json!(obj.version));
    data
}

#[cfg(test)]
#[path = "ws_test.rs"]
mod tests;
