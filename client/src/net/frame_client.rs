//! WebSocket frame client for real-time communication with the server.
//!
//! The `FrameClient` manages the WebSocket lifecycle: connection, reconnection
//! with exponential backoff, frame dispatch, and signal updates. It is the
//! primary bridge between the server's frame protocol and the Leptos UI state.
//!
//! All WebSocket logic is gated behind `#[cfg(feature = "hydrate")]` since it
//! requires a browser environment.
//!
//! ERROR HANDLING
//! ==============
//! Parse/transport failures are handled defensively and translated into state
//! updates/logging so realtime UX can recover through reconnect loops.

#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::ai::AiMessage;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::ai::AiState;
#[cfg(feature = "hydrate")]
use crate::state::auth::AuthState;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use crate::state::board::ConnectionStatus;
#[cfg(feature = "hydrate")]
use crate::state::boards::BoardsState;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::boards::{BoardListItem, BoardListPreviewObject};
#[cfg(any(test, feature = "hydrate"))]
use crate::state::chat::ChatMessage;
#[cfg(feature = "hydrate")]
use crate::state::chat::ChatState;
#[cfg(feature = "hydrate")]
use crate::state::trace::TraceState;
#[cfg(feature = "hydrate")]
use leptos::prelude::GetUntracked;
#[cfg(feature = "hydrate")]
use leptos::prelude::Update;

/// Send a frame to the server via the shared sender channel.
///
/// Returns `false` if the channel is closed (no active connection).
#[cfg(feature = "hydrate")]
pub fn send_frame(tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>, frame: &Frame) -> bool {
    tx.unbounded_send(frames::encode_frame(frame)).is_ok()
}

/// Spawn the WebSocket frame client lifecycle as a local async task.
///
/// This connects to the server, handles incoming frames, and reconnects
/// on disconnect with exponential backoff.
#[cfg(feature = "hydrate")]
pub fn spawn_frame_client(
    auth: leptos::prelude::RwSignal<AuthState>,
    ai: leptos::prelude::RwSignal<AiState>,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    trace: leptos::prelude::RwSignal<TraceState>,
) -> futures::channel::mpsc::UnboundedSender<Vec<u8>> {
    use futures::channel::mpsc;

    let (tx, rx) = mpsc::unbounded::<Vec<u8>>();
    let tx_clone = tx.clone();

    leptos::task::spawn_local(frame_client_loop(auth, ai, board, boards, chat, trace, tx_clone, rx));

    tx
}

/// Main connection loop with reconnect logic.
#[cfg(feature = "hydrate")]
async fn frame_client_loop(
    auth: leptos::prelude::RwSignal<AuthState>,
    ai: leptos::prelude::RwSignal<AiState>,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    trace: leptos::prelude::RwSignal<TraceState>,
    tx: futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    rx: futures::channel::mpsc::UnboundedReceiver<Vec<u8>>,
) {
    use std::cell::RefCell;
    use std::rc::Rc;

    let rx = Rc::new(RefCell::new(rx));
    let mut backoff_ms: u32 = 1000;
    let max_backoff_ms: u32 = 10_000;

    loop {
        board.update(|b| b.connection_status = ConnectionStatus::Connecting);

        // Get a WS ticket.
        let ticket = match crate::net::api::create_ws_ticket().await {
            Ok(t) => t,
            Err(e) => {
                leptos::logging::warn!("WS ticket failed: {e}");
                gloo_timers::future::sleep(std::time::Duration::from_millis(u64::from(backoff_ms))).await;
                backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                continue;
            }
        };

        // Determine WebSocket URL.
        let location = web_sys::window()
            .and_then(|w| w.location().href().ok())
            .unwrap_or_default();
        let ws_proto = if location.starts_with("https") { "wss" } else { "ws" };
        let host = web_sys::window()
            .and_then(|w| w.location().host().ok())
            .unwrap_or_else(|| "localhost:3000".to_owned());
        let ws_url = format!("{ws_proto}://{host}/api/ws?ticket={ticket}");

        match connect_and_run(&ws_url, auth, ai, board, boards, chat, trace, &tx, &rx).await {
            Ok(()) => {
                leptos::logging::log!("WS disconnected cleanly");
            }
            Err(e) => {
                leptos::logging::warn!("WS error: {e}");
            }
        }

        board.update(|b| b.connection_status = ConnectionStatus::Disconnected);

        // Exponential backoff before reconnect.
        gloo_timers::future::sleep(std::time::Duration::from_millis(u64::from(backoff_ms))).await;
        backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
    }
}

/// Connect to the WebSocket and process messages until disconnect.
#[cfg(feature = "hydrate")]
async fn connect_and_run(
    url: &str,
    auth: leptos::prelude::RwSignal<AuthState>,
    ai: leptos::prelude::RwSignal<AiState>,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    trace: leptos::prelude::RwSignal<TraceState>,
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    rx: &std::rc::Rc<std::cell::RefCell<futures::channel::mpsc::UnboundedReceiver<Vec<u8>>>>,
) -> Result<(), String> {
    use futures::StreamExt;
    use gloo_net::websocket::Message;
    use gloo_net::websocket::futures::WebSocket;

    let ws = WebSocket::open(url).map_err(|e| e.to_string())?;
    let (mut ws_write, mut ws_read) = ws.split();

    board.update(|b| b.connection_status = ConnectionStatus::Connected);

    // Spawn a task to forward outgoing messages from our channel to the WS.
    let mut rx_borrow = rx.borrow_mut();
    let send_task = async {
        use futures::SinkExt;
        while let Some(msg) = rx_borrow.next().await {
            if ws_write.send(Message::Bytes(msg)).await.is_err() {
                break;
            }
        }
    };

    // Receive loop: process incoming frames.
    let recv_task = async {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Bytes(bytes)) => {
                    if let Ok(frame) = frames::decode_frame(&bytes) {
                        dispatch_frame(&frame, auth, ai, board, boards, chat, trace, tx);
                    }
                }
                Ok(Message::Text(_)) => {}
                Err(e) => {
                    leptos::logging::warn!("WS recv error: {e}");
                    break;
                }
            }
        }
    };

    // Run send/recv loops; when either finishes, the connection is done.
    let io_task = async {
        futures::future::select(Box::pin(send_task), Box::pin(recv_task)).await;
    };
    io_task.await;

    Ok(())
}

/// Dispatch an incoming frame to the appropriate state handler.
///
/// Every frame is also appended to the trace buffer unconditionally so the
/// observability view can render the full event stream.
#[cfg(feature = "hydrate")]
fn dispatch_frame(
    frame: &Frame,
    _auth: leptos::prelude::RwSignal<AuthState>,
    ai: leptos::prelude::RwSignal<AiState>,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    trace: leptos::prelude::RwSignal<TraceState>,
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
) {
    // Buffer every frame for the observability view before domain routing.
    trace.update(|t| t.push_frame(frame.clone()));

    if handle_session_connected_frame(frame, board, boards, tx) {
        return;
    }
    if handle_board_frame(frame, board, boards, tx) {
        return;
    }
    if handle_object_frame(frame, board) {
        return;
    }
    if handle_chat_frame(frame, chat) {
        return;
    }
    if handle_ai_frame(frame, ai) {
        return;
    }
    if handle_error_frame(frame, boards) {
        return;
    }
    if frame.syscall == "gateway:error" {
        leptos::logging::warn!("gateway:error frame: {}", frame.data);
    }
}

#[cfg(feature = "hydrate")]
fn handle_session_connected_frame(
    frame: &Frame,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
) -> bool {
    if frame.syscall != "session:connected" {
        return false;
    }
    board.update(|b| {
        b.connection_status = ConnectionStatus::Connected;
        b.self_client_id = frame
            .data
            .get("client_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
    });
    send_board_list_request(tx, boards);
    send_board_users_list_request(tx, board);
    true
}

#[cfg(feature = "hydrate")]
fn handle_board_frame(
    frame: &Frame,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
) -> bool {
    use crate::net::types::{FrameStatus, Savepoint};

    let op = frame.syscall.strip_prefix("board:");
    match op {
        Some("join") if frame.status == FrameStatus::Item => {
            if let Some(obj) = parse_board_object_item(&frame.data) {
                board.update(|b| {
                    if !b.join_streaming {
                        b.objects.clear();
                        b.drag_objects.clear();
                        b.drag_updated_at.clear();
                        b.join_streaming = true;
                    }
                    b.objects.insert(obj.id.clone(), obj);
                });
            }
            true
        }
        Some("join") if frame.status == FrameStatus::Done => {
            board.update(|b| {
                if let Some(objs) = parse_board_objects(&frame.data) {
                    b.objects.clear();
                    b.drag_objects.clear();
                    b.drag_updated_at.clear();
                    for obj in objs {
                        b.objects.insert(obj.id.clone(), obj);
                    }
                } else if !b.join_streaming {
                    // Empty stream: clear stale data from prior board snapshot.
                    b.objects.clear();
                    b.drag_objects.clear();
                    b.drag_updated_at.clear();
                }
                b.join_streaming = false;
            });
            if let Some(name) = frame.data.get("name").and_then(|n| n.as_str()) {
                board.update(|b| b.board_name = Some(name.to_owned()));
            }
            send_board_savepoint_list_request(tx, board);
            send_board_users_list_request(tx, board);
            true
        }
        Some("join") => {
            if frame.data.get("client_id").is_some() && frame.data.get("objects").is_none() {
                board.update(|b| {
                    upsert_presence_from_payload(b, &frame.data);
                });
            }
            true
        }
        Some("users:list") if frame.status == FrameStatus::Done => {
            if let Some(rows) = frame.data.get("users").and_then(|v| v.as_array()) {
                board.update(|b| {
                    b.presence.clear();
                    b.cursor_updated_at.clear();
                    for row in rows {
                        upsert_presence_from_payload(b, row);
                    }
                });
            }
            true
        }
        Some("list") if frame.status == FrameStatus::Done => {
            let noop = frame
                .data
                .get("noop")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let rev = frame
                .data
                .get("rev")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            boards.update(|s| {
                if !noop {
                    s.items = parse_board_list_items(&frame.data);
                }
                s.list_rev = rev;
                s.loading = false;
                s.error = None;
            });
            true
        }
        Some("list:refresh") => {
            boards.update(|s| s.loading = true);
            send_board_list_request(tx, boards);
            true
        }
        Some("create") if frame.status == FrameStatus::Done => {
            if let Ok(created) = serde_json::from_value::<BoardListItem>(frame.data.clone()) {
                boards.update(|s| {
                    if let Some(existing) = s.items.iter_mut().find(|b| b.id == created.id) {
                        *existing = created.clone();
                    } else {
                        s.items.insert(0, created.clone());
                    }
                    s.create_pending = false;
                    s.created_board_id = Some(created.id.clone());
                    s.error = None;
                });
                send_board_list_request(tx, boards);
            } else {
                boards.update(|s| s.create_pending = false);
            }
            true
        }
        Some("delete") if frame.status == FrameStatus::Done => {
            handle_deleted_board_eject(frame, board);
            send_board_list_request(tx, boards);
            true
        }
        Some("delete") => {
            handle_deleted_board_eject(frame, board);
            send_board_list_request(tx, boards);
            true
        }
        Some("savepoint:list") if frame.status == FrameStatus::Done => {
            let savepoints = frame
                .data
                .get("savepoints")
                .cloned()
                .and_then(|v| serde_json::from_value::<Vec<Savepoint>>(v).ok())
                .unwrap_or_default();
            board.update(|b| b.savepoints = savepoints);
            true
        }
        Some("savepoint:create") if frame.status == FrameStatus::Done => {
            if let Some(value) = frame.data.get("savepoint")
                && let Ok(created) = serde_json::from_value::<Savepoint>(value.clone())
            {
                board.update(|b| {
                    if let Some(existing) = b.savepoints.iter_mut().find(|s| s.id == created.id) {
                        *existing = created.clone();
                    } else {
                        b.savepoints.insert(0, created.clone());
                    }
                    b.savepoints.sort_by(|a, c| c.seq.cmp(&a.seq));
                });
            } else {
                send_board_savepoint_list_request(tx, board);
            }
            true
        }
        Some("part") => {
            if let Some(client_id) = frame.data.get("client_id").and_then(|v| v.as_str()) {
                board.update(|b| {
                    b.presence.remove(client_id);
                    b.cursor_updated_at.remove(client_id);
                });
            }
            true
        }
        Some("access:generate") if frame.status == FrameStatus::Done => {
            if let Some(code) = frame.data.get("code").and_then(|v| v.as_str()) {
                board.update(|b| b.generated_access_code = Some(code.to_owned()));
            }
            true
        }
        Some("access:redeem") if frame.status == FrameStatus::Done => {
            if let Some(board_id) = frame.data.get("board_id").and_then(|v| v.as_str()) {
                boards.update(|s| s.redeemed_board_id = Some(board_id.to_owned()));
            }
            true
        }
        _ => false,
    }
}

#[cfg(feature = "hydrate")]
fn handle_deleted_board_eject(frame: &Frame, board: leptos::prelude::RwSignal<BoardState>) {
    if let Some(deleted_board_id) = deleted_board_id(frame)
        && board.get_untracked().board_id.as_deref() == Some(deleted_board_id.as_str())
    {
        board.update(|b| {
            b.board_id = None;
            b.board_name = None;
            b.follow_client_id = None;
            b.jump_to_client_id = None;
            b.objects.clear();
            b.savepoints.clear();
            b.drag_objects.clear();
            b.drag_updated_at.clear();
            b.cursor_updated_at.clear();
            b.join_streaming = false;
            b.selection.clear();
            b.presence.clear();
        });
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/");
        }
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_board_list_items(data: &serde_json::Value) -> Vec<BoardListItem> {
    let Some(rows) = data.get("boards").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    rows.iter()
        .filter_map(|row| {
            let id = row.get("id")?.as_str()?.to_owned();
            let name = row.get("name")?.as_str()?.to_owned();
            let owner_id = row
                .get("owner_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let snapshot = row
                .get("snapshot")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(parse_board_list_preview_object)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(BoardListItem {
                id,
                name,
                owner_id,
                snapshot,
            })
        })
        .collect()
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_board_list_preview_object(row: &serde_json::Value) -> Option<BoardListPreviewObject> {
    let kind = row
        .get("kind")
        .and_then(serde_json::Value::as_str)?
        .to_owned();
    let x = row.get("x").and_then(serde_json::Value::as_f64)?;
    let y = row.get("y").and_then(serde_json::Value::as_f64)?;
    let width = row.get("width").and_then(serde_json::Value::as_f64);
    let height = row.get("height").and_then(serde_json::Value::as_f64);
    let rotation = row
        .get("rotation")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let z_index = row
        .get("z_index")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|n| n.round() as i64))
        })
        .and_then(|n| i32::try_from(n).ok())
        .unwrap_or(0);
    Some(BoardListPreviewObject { kind, x, y, width, height, rotation, z_index })
}

#[cfg(any(test, feature = "hydrate"))]
fn deleted_board_id(frame: &Frame) -> Option<String> {
    frame
        .data
        .get("board_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| frame.board_id.clone())
}

#[cfg(feature = "hydrate")]
fn handle_object_frame(frame: &Frame, board: leptos::prelude::RwSignal<BoardState>) -> bool {
    board.update(|b| {
        apply_object_frame(frame, b);
    });
    matches!(
        frame.syscall.as_str(),
        "object:create"
            | "object:update"
            | "object:delete"
            | "object:drag"
            | "object:drag:end"
            | "cursor:moved"
            | "cursor:clear"
    )
}

#[cfg(any(test, feature = "hydrate"))]
fn apply_object_frame(frame: &Frame, board: &mut BoardState) {
    use crate::net::types::{BoardObject, FrameStatus};
    cleanup_stale_drags(board, frame.ts);
    cleanup_stale_cursors(board, frame.ts);

    match frame.syscall.as_str() {
        "object:create" if frame.status == FrameStatus::Done => {
            if let Ok(obj) = serde_json::from_value::<BoardObject>(frame.data.clone()) {
                board.objects.insert(obj.id.clone(), obj);
            }
        }
        "object:update" if frame.status == FrameStatus::Done => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                if let Some(existing) = board.objects.get_mut(id) {
                    merge_object_update(existing, &frame.data);
                    board.drag_objects.remove(id);
                    board.drag_updated_at.remove(id);
                } else {
                    // Defensive: don't keep stale selection for unknown objects.
                    board.selection.remove(id);
                }
            }
        }
        "object:delete" if frame.status == FrameStatus::Done => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                board.objects.remove(id);
                board.selection.remove(id);
                board.drag_objects.remove(id);
                board.drag_updated_at.remove(id);
            }
        }
        "object:drag" => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str())
                && let Some(existing) = board.objects.get(id)
            {
                // Conflict guard: don't apply peer drag jitter onto local selected object.
                if board.selection.contains(id) {
                    return;
                }
                let mut dragged = existing.clone();
                merge_object_update(&mut dragged, &frame.data);
                if let Some(prev) = board.drag_objects.get(id) {
                    let prev_ts = board.drag_updated_at.get(id).copied().unwrap_or(frame.ts);
                    if should_smooth_drag(prev_ts, frame.ts) {
                        smooth_drag_object(prev, &mut dragged, &frame.data, smoothing_alpha(prev_ts, frame.ts));
                    }
                }
                board.drag_objects.insert(id.to_owned(), dragged);
                board.drag_updated_at.insert(id.to_owned(), frame.ts);
            }
        }
        "object:drag:end" => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                board.drag_objects.remove(id);
                board.drag_updated_at.remove(id);
            }
        }
        "cursor:moved" => apply_cursor_moved(board, &frame.data, frame.ts),
        "cursor:clear" => apply_cursor_clear(board, &frame.data),
        _ => {}
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn smooth_drag_object(
    previous: &crate::net::types::BoardObject,
    next: &mut crate::net::types::BoardObject,
    patch: &serde_json::Value,
    alpha: f64,
) {
    if patch.get("x").is_some() {
        next.x = lerp(previous.x, next.x, alpha);
    }
    if patch.get("y").is_some() {
        next.y = lerp(previous.y, next.y, alpha);
    }
    if patch.get("width").is_some()
        && let (Some(prev), Some(curr)) = (previous.width, next.width)
    {
        next.width = Some(lerp(prev, curr, alpha));
    }
    if patch.get("height").is_some()
        && let (Some(prev), Some(curr)) = (previous.height, next.height)
    {
        next.height = Some(lerp(prev, curr, alpha));
    }
    if patch.get("rotation").is_some() {
        next.rotation = lerp(previous.rotation, next.rotation, alpha);
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(any(test, feature = "hydrate"))]
fn should_smooth_drag(prev_ts: i64, next_ts: i64) -> bool {
    // Keep fast streams crisp; smooth only slower arrivals.
    next_ts.saturating_sub(prev_ts) >= 80
}

#[cfg(any(test, feature = "hydrate"))]
fn smoothing_alpha(prev_ts: i64, next_ts: i64) -> f64 {
    let dt = next_ts.saturating_sub(prev_ts);
    if dt >= 200 {
        0.65
    } else if dt >= 120 {
        0.55
    } else {
        0.45
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn cleanup_stale_drags(board: &mut BoardState, now_ts: i64) {
    const DRAG_STALE_MS: i64 = 1500;
    if now_ts <= 0 {
        return;
    }
    let stale = board
        .drag_updated_at
        .iter()
        .filter_map(|(id, ts)| (now_ts - *ts > DRAG_STALE_MS).then_some(id.clone()))
        .collect::<Vec<_>>();
    for id in stale {
        board.drag_updated_at.remove(&id);
        board.drag_objects.remove(&id);
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn cleanup_stale_cursors(board: &mut BoardState, now_ts: i64) {
    const CURSOR_STALE_MS: i64 = 3000;
    if now_ts <= 0 {
        return;
    }
    let stale = board
        .cursor_updated_at
        .iter()
        .filter_map(|(id, ts)| (now_ts - *ts > CURSOR_STALE_MS).then_some(id.clone()))
        .collect::<Vec<_>>();
    for id in stale {
        board.cursor_updated_at.remove(&id);
        if let Some(p) = board.presence.get_mut(&id) {
            p.cursor = None;
        }
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn apply_cursor_moved(board: &mut BoardState, data: &serde_json::Value, ts: i64) {
    use crate::net::types::Point;

    let Some(client_id) = data.get("client_id").and_then(|v| v.as_str()) else {
        return;
    };
    let x = data.get("x").and_then(|v| v.as_f64());
    let y = data.get("y").and_then(|v| v.as_f64());
    let camera_center_x = data.get("camera_center_x").and_then(|v| v.as_f64());
    let camera_center_y = data.get("camera_center_y").and_then(|v| v.as_f64());
    let camera_zoom = data.get("camera_zoom").and_then(|v| v.as_f64());
    let camera_rotation = data.get("camera_rotation").and_then(|v| v.as_f64());

    if !board.presence.contains_key(client_id) {
        upsert_presence_from_payload(board, data);
    }
    if let Some(p) = board.presence.get_mut(client_id) {
        if let (Some(x), Some(y)) = (x, y) {
            board.cursor_updated_at.insert(client_id.to_owned(), ts);
            p.cursor = Some(Point { x, y });
        }
        if let (Some(cx), Some(cy)) = (camera_center_x, camera_center_y) {
            p.camera_center = Some(Point { x: cx, y: cy });
        }
        if let Some(zoom) = camera_zoom {
            p.camera_zoom = Some(zoom);
        }
        if let Some(rotation) = camera_rotation {
            p.camera_rotation = Some(rotation);
        }
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn apply_cursor_clear(board: &mut BoardState, data: &serde_json::Value) {
    let Some(client_id) = data.get("client_id").and_then(|v| v.as_str()) else {
        return;
    };
    board.cursor_updated_at.remove(client_id);
    if let Some(p) = board.presence.get_mut(client_id) {
        p.cursor = None;
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn upsert_presence_from_payload(board: &mut BoardState, data: &serde_json::Value) {
    use crate::net::types::Presence;

    let Some(client_id) = data.get("client_id").and_then(|v| v.as_str()) else {
        return;
    };
    let user_id = data
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or(client_id);
    let user_name = data
        .get("user_name")
        .or_else(|| data.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Agent");
    let user_color = data
        .get("user_color")
        .or_else(|| data.get("color"))
        .and_then(|v| v.as_str())
        .unwrap_or("#8a8178");

    let existing_cursor = board.presence.get(client_id).and_then(|p| p.cursor.clone());
    let existing_camera_center = board
        .presence
        .get(client_id)
        .and_then(|p| p.camera_center.clone());
    let existing_camera_zoom = board.presence.get(client_id).and_then(|p| p.camera_zoom);
    let existing_camera_rotation = board
        .presence
        .get(client_id)
        .and_then(|p| p.camera_rotation);
    let payload_camera_center = data
        .get("camera_center")
        .and_then(|v| serde_json::from_value::<crate::net::types::Point>(v.clone()).ok())
        .or_else(|| {
            Some(crate::net::types::Point {
                x: data.get("camera_center_x")?.as_f64()?,
                y: data.get("camera_center_y")?.as_f64()?,
            })
        });
    let payload_camera_zoom = data.get("camera_zoom").and_then(|v| v.as_f64());
    let payload_camera_rotation = data.get("camera_rotation").and_then(|v| v.as_f64());
    board.presence.insert(
        client_id.to_owned(),
        Presence {
            client_id: client_id.to_owned(),
            user_id: user_id.to_owned(),
            name: user_name.to_owned(),
            color: user_color.to_owned(),
            cursor: existing_cursor,
            camera_center: payload_camera_center.or(existing_camera_center),
            camera_zoom: payload_camera_zoom.or(existing_camera_zoom),
            camera_rotation: payload_camera_rotation.or(existing_camera_rotation),
        },
    );
}

#[cfg(feature = "hydrate")]
fn handle_chat_frame(frame: &Frame, chat: leptos::prelude::RwSignal<ChatState>) -> bool {
    use crate::net::types::FrameStatus;

    match frame.syscall.as_str() {
        "chat:message" if frame.status == FrameStatus::Done => {
            if let Some(msg) = parse_chat_message(frame, &frame.data) {
                chat.update(|c| c.messages.push(msg));
            }
            true
        }
        "chat:history" if frame.status == FrameStatus::Done => {
            if let Some(messages) = frame.data.get("messages") {
                let list = messages
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| parse_chat_message(frame, item))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                chat.update(|c| c.messages = list);
            }
            true
        }
        _ => false,
    }
}

#[cfg(feature = "hydrate")]
fn handle_ai_frame(frame: &Frame, ai: leptos::prelude::RwSignal<AiState>) -> bool {
    use crate::net::types::FrameStatus;

    match frame.syscall.as_str() {
        "ai:history" if frame.status == FrameStatus::Done => {
            if let Some(messages) = frame.data.get("messages") {
                let list = messages
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(parse_ai_message_value)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                ai.update(|a| {
                    a.messages = list;
                    a.loading = false;
                });
            }
            true
        }
        "ai:prompt" if frame.status == FrameStatus::Done || frame.status == FrameStatus::Error => {
            if let Some(user_msg) = parse_ai_prompt_user_message(frame) {
                ai.update(|a| upsert_ai_user_message(a, user_msg));
            }
            if let Some(msg) = parse_ai_prompt_message(frame) {
                ai.update(|a| {
                    a.messages.push(msg);
                    a.loading = false;
                });
            } else if frame.status == FrameStatus::Error {
                let content = frame_error_message(frame)
                    .unwrap_or("AI request failed")
                    .to_owned();
                ai.update(|a| {
                    a.messages.push(AiMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        role: "error".to_owned(),
                        content,
                        timestamp: 0.0,
                        mutations: None,
                    });
                    a.loading = false;
                });
            } else {
                ai.update(|a| a.loading = false);
            }
            true
        }
        _ => false,
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn upsert_ai_user_message(ai: &mut AiState, msg: AiMessage) {
    if let Some(existing) = ai
        .messages
        .iter_mut()
        .find(|existing| existing.id == msg.id && existing.role == "user")
    {
        existing.content = msg.content;
        if existing.timestamp == 0.0 {
            existing.timestamp = msg.timestamp;
        }
        return;
    }
    ai.messages.push(msg);
}

#[cfg(feature = "hydrate")]
fn handle_error_frame(frame: &Frame, boards: leptos::prelude::RwSignal<BoardsState>) -> bool {
    use crate::net::types::FrameStatus;

    if frame.status != FrameStatus::Error {
        return false;
    }

    let message = frame_error_message(frame)
        .unwrap_or("request failed")
        .to_owned();
    if frame.syscall == "board:list" {
        boards.update(|s| {
            s.loading = false;
            s.error = Some(message.clone());
        });
    } else if frame.syscall == "board:create" {
        boards.update(|s| {
            s.create_pending = false;
            s.error = Some(message.clone());
        });
    } else if frame.syscall == "board:delete" {
        boards.update(|s| {
            s.loading = false;
            s.error = Some(message.clone());
        });
    }
    leptos::logging::warn!("frame error: syscall={} data={}", frame.syscall, frame.data);
    true
}

#[cfg(feature = "hydrate")]
fn send_board_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    boards: leptos::prelude::RwSignal<BoardsState>,
) {
    let since_rev = boards.get_untracked().list_rev;
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({
            "since_rev": since_rev
        }),
    };
    let _ = send_frame(tx, &frame);
}

#[cfg(feature = "hydrate")]
fn send_board_savepoint_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    board: leptos::prelude::RwSignal<BoardState>,
) {
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "board:savepoint:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    };
    let _ = send_frame(tx, &frame);
}

#[cfg(feature = "hydrate")]
fn send_board_users_list_request(
    tx: &futures::channel::mpsc::UnboundedSender<Vec<u8>>,
    board: leptos::prelude::RwSignal<BoardState>,
) {
    let Some(board_id) = board.get_untracked().board_id else {
        return;
    };
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id),
        from: None,
        syscall: "board:users:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    };
    let _ = send_frame(tx, &frame);
}

/// Merge partial object updates into an existing `BoardObject`.
#[cfg(any(test, feature = "hydrate"))]
fn merge_object_update(obj: &mut crate::net::types::BoardObject, data: &serde_json::Value) {
    if let Some(x) = data.get("x").and_then(|v| v.as_f64()) {
        obj.x = x;
    }
    if let Some(y) = data.get("y").and_then(|v| v.as_f64()) {
        obj.y = y;
    }
    if let Some(w) = data.get("width").and_then(|v| v.as_f64()) {
        obj.width = Some(w);
    }
    if let Some(h) = data.get("height").and_then(|v| v.as_f64()) {
        obj.height = Some(h);
    }
    if let Some(r) = data.get("rotation").and_then(|v| v.as_f64()) {
        obj.rotation = r;
    }
    if let Some(z) = data.get("z_index").and_then(number_as_i64) {
        #[allow(clippy::cast_possible_truncation)]
        {
            obj.z_index = z as i32;
        }
    }
    if let Some(props) = data.get("props") {
        obj.props = props.clone();
    }
    if let Some(v) = data.get("version").and_then(number_as_i64) {
        obj.version = v;
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_board_objects(data: &serde_json::Value) -> Option<Vec<crate::net::types::BoardObject>> {
    data.get("objects")
        .cloned()
        .and_then(|v| serde_json::from_value::<Vec<crate::net::types::BoardObject>>(v).ok())
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_board_object_item(data: &serde_json::Value) -> Option<crate::net::types::BoardObject> {
    serde_json::from_value::<crate::net::types::BoardObject>(data.clone()).ok()
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_chat_message(frame: &Frame, data: &serde_json::Value) -> Option<ChatMessage> {
    let content = pick_str(data, &["content", "message"])?.to_owned();

    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| frame.id.clone());

    let user_id = pick_str(data, &["user_id", "from"])
        .or(frame.from.as_deref())
        .unwrap_or("unknown")
        .to_owned();

    let user_name = data
        .get("user_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Agent")
        .to_owned();

    let user_color = data
        .get("user_color")
        .and_then(|v| v.as_str())
        .unwrap_or("#8a8178")
        .to_owned();

    let timestamp = pick_number(data, &["timestamp", "ts"]).unwrap_or(frame.ts as f64);

    Some(ChatMessage { id, user_id, user_name, user_color, content, timestamp })
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_ai_message_value(data: &serde_json::Value) -> Option<AiMessage> {
    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("ai-msg")
        .to_owned();
    let role = data
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("assistant")
        .to_owned();
    let content = pick_str(data, &["content", "text"])
        .unwrap_or_default()
        .to_owned();
    if content.trim().is_empty() {
        return None;
    }
    let timestamp = pick_number(data, &["timestamp", "ts"]).unwrap_or(0.0);
    let mutations = data.get("mutations").and_then(number_as_i64);

    Some(AiMessage { id, role, content, timestamp, mutations })
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_ai_prompt_message(frame: &Frame) -> Option<AiMessage> {
    if let Some(mut msg) = parse_ai_message_value(&frame.data) {
        if frame.status == crate::net::types::FrameStatus::Error && msg.role == "assistant" {
            msg.role = "error".to_owned();
        }
        if msg.timestamp == 0.0 {
            msg.timestamp = frame.ts as f64;
        }
        return Some(msg);
    }

    let content = pick_str(&frame.data, &["text", "content"])?;

    Some(AiMessage {
        id: frame.id.clone(),
        role: if frame.status == crate::net::types::FrameStatus::Error {
            "error".to_owned()
        } else {
            "assistant".to_owned()
        },
        content: content.to_owned(),
        timestamp: frame.ts as f64,
        mutations: frame.data.get("mutations").and_then(number_as_i64),
    })
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_ai_prompt_user_message(frame: &Frame) -> Option<AiMessage> {
    let prompt = frame
        .data
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)?;
    if prompt.is_empty() {
        return None;
    }

    Some(AiMessage {
        // ai:prompt done/error replies carry a new id and set parent_id to the original request id.
        // Use parent_id so optimistic user rows reconcile instead of duplicating.
        id: frame.parent_id.clone().unwrap_or_else(|| frame.id.clone()),
        role: "user".to_owned(),
        content: prompt.to_owned(),
        timestamp: frame.ts as f64,
        mutations: None,
    })
}

#[cfg(any(test, feature = "hydrate"))]
fn frame_error_message(frame: &Frame) -> Option<&str> {
    pick_str(&frame.data, &["message", "error"])
}

#[cfg(any(test, feature = "hydrate"))]
fn pick_str<'a>(data: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = data.get(key).and_then(serde_json::Value::as_str) {
            return Some(value);
        }
    }
    None
}

#[cfg(any(test, feature = "hydrate"))]
fn pick_number(data: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(value) = data.get(key) {
            if let Some(n) = value.as_f64() {
                return Some(n);
            }
            if let Some(n) = value.as_i64() {
                #[allow(clippy::cast_precision_loss)]
                {
                    return Some(n as f64);
                }
            }
        }
    }
    None
}

#[cfg(any(test, feature = "hydrate"))]
fn number_as_i64(value: &serde_json::Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_f64()
            .filter(|v| v.is_finite() && v.fract() == 0.0)
            .and_then(|v| {
                if (i64::MIN as f64..=i64::MAX as f64).contains(&v) {
                    Some(v as i64)
                } else {
                    None
                }
            })
    })
}

#[cfg(test)]
#[path = "frame_client_test.rs"]
mod frame_client_test;
