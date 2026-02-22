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

#[path = "frame_client_ai.rs"]
mod frame_client_ai;
#[path = "frame_client_chat.rs"]
mod frame_client_chat;
#[path = "frame_client_error.rs"]
mod frame_client_error;
#[path = "frame_client_objects.rs"]
mod frame_client_objects;
#[path = "frame_client_parse.rs"]
mod frame_client_parse;
#[path = "frame_client_requests.rs"]
mod frame_client_requests;

#[cfg(feature = "hydrate")]
use self::frame_client_ai::handle_ai_frame;
#[cfg(feature = "hydrate")]
use self::frame_client_chat::handle_chat_frame;
#[cfg(feature = "hydrate")]
use self::frame_client_error::handle_error_frame;
#[cfg(feature = "hydrate")]
use self::frame_client_objects::handle_object_frame;
#[cfg(feature = "hydrate")]
use self::frame_client_requests::{
    send_board_list_request, send_board_savepoint_list_request, send_board_users_list_request,
};

#[cfg(test)]
use self::frame_client_objects::{
    apply_cursor_clear, apply_cursor_moved, apply_object_frame, cleanup_stale_cursors, cleanup_stale_drags,
    merge_object_update, should_smooth_drag, smoothing_alpha,
};

#[cfg(any(test, feature = "hydrate"))]
use self::frame_client_parse::{
    deleted_board_id, frame_error_message, parse_ai_message_value, parse_ai_prompt_message,
    parse_ai_prompt_user_message, parse_board_list_items, parse_board_object_item, parse_board_objects,
    parse_chat_message,
};
#[cfg(feature = "hydrate")]
use crate::net::types::{BoardObject, Frame, FrameStatus};
#[cfg(any(test, feature = "hydrate"))]
use crate::state::ai::AiState;
#[cfg(feature = "hydrate")]
use crate::state::auth::AuthState;
#[cfg(feature = "hydrate")]
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use crate::state::board::ConnectionStatus;
#[cfg(feature = "hydrate")]
use crate::state::boards::BoardListItem;
#[cfg(feature = "hydrate")]
use crate::state::boards::BoardsState;
#[cfg(feature = "hydrate")]
use crate::state::chat::ChatState;
#[cfg(feature = "hydrate")]
use crate::state::trace::TraceState;
#[cfg(feature = "hydrate")]
use leptos::prelude::GetUntracked;
#[cfg(feature = "hydrate")]
use leptos::prelude::Update;
#[cfg(feature = "hydrate")]
use std::cell::RefCell;

#[cfg(feature = "hydrate")]
thread_local! {
    static JOIN_ITEM_BUFFER: RefCell<Vec<BoardObject>> = const { RefCell::new(Vec::new()) };
    static LIVE_OBJECT_FRAME_BUFFER: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static LIVE_OBJECT_FLUSH_SCHEDULED: RefCell<bool> = const { RefCell::new(false) };
    static TRACE_MUTED_UNTIL_JOIN_DONE: RefCell<bool> = const { RefCell::new(false) };
}

#[cfg(feature = "hydrate")]
const LIVE_OBJECT_FLUSH_BATCH_SIZE: usize = 256;

#[cfg(feature = "hydrate")]
fn flush_join_items(board: leptos::prelude::RwSignal<BoardState>) {
    let drained = JOIN_ITEM_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        if buf.is_empty() {
            Vec::new()
        } else {
            buf.drain(..).collect::<Vec<_>>()
        }
    });
    if drained.is_empty() {
        return;
    }

    board.update(|b| {
        for obj in drained {
            b.objects.insert(obj.id.clone(), obj);
        }
        b.bump_scene_rev();
    });
}

#[cfg(feature = "hydrate")]
fn queue_join_item(board: leptos::prelude::RwSignal<BoardState>, obj: BoardObject) {
    let _ = board;
    JOIN_ITEM_BUFFER.with(|buf| buf.borrow_mut().push(obj));
}

#[cfg(feature = "hydrate")]
fn flush_live_object_frames(board: leptos::prelude::RwSignal<BoardState>) {
    let drained = LIVE_OBJECT_FRAME_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        if buf.is_empty() {
            Vec::new()
        } else {
            buf.drain(..).collect::<Vec<_>>()
        }
    });
    if drained.is_empty() {
        return;
    }

    board.update(|b| {
        for frame in drained {
            frame_client_objects::apply_object_frame(&frame, b);
        }
    });
}

#[cfg(feature = "hydrate")]
fn schedule_live_object_flush(board: leptos::prelude::RwSignal<BoardState>) {
    let already_scheduled = LIVE_OBJECT_FLUSH_SCHEDULED.with(|flag| {
        let mut flag = flag.borrow_mut();
        if *flag {
            true
        } else {
            *flag = true;
            false
        }
    });
    if already_scheduled {
        return;
    }

    leptos::task::spawn_local(async move {
        gloo_timers::future::sleep(std::time::Duration::from_millis(16)).await;
        flush_live_object_frames(board);
        LIVE_OBJECT_FLUSH_SCHEDULED.with(|flag| *flag.borrow_mut() = false);
    });
}

#[cfg(feature = "hydrate")]
fn queue_live_object_frame(board: leptos::prelude::RwSignal<BoardState>, frame: &Frame) -> bool {
    let is_batchable = matches!(
        (frame.syscall.as_str(), frame.status),
        ("object:create", crate::net::types::FrameStatus::Done)
            | ("object:update", crate::net::types::FrameStatus::Done)
            | ("object:delete", crate::net::types::FrameStatus::Done)
    );
    if !is_batchable {
        return false;
    }

    let flush_now = LIVE_OBJECT_FRAME_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.push(frame.clone());
        buf.len() >= LIVE_OBJECT_FLUSH_BATCH_SIZE
    });
    if flush_now {
        flush_live_object_frames(board);
        LIVE_OBJECT_FLUSH_SCHEDULED.with(|flag| *flag.borrow_mut() = false);
    } else {
        schedule_live_object_flush(board);
    }
    true
}

#[cfg(feature = "hydrate")]
fn should_record_trace(frame: &Frame) -> bool {
    // Cursor events are interaction telemetry, not observability trace data.
    if frame.syscall.starts_with("cursor:") {
        return false;
    }

    if frame.syscall == "board:join" {
        match frame.status {
            FrameStatus::Item => {
                TRACE_MUTED_UNTIL_JOIN_DONE.with(|flag| *flag.borrow_mut() = true);
                return false;
            }
            FrameStatus::Done => {
                TRACE_MUTED_UNTIL_JOIN_DONE.with(|flag| *flag.borrow_mut() = false);
                return false;
            }
            _ => return false,
        }
    }

    TRACE_MUTED_UNTIL_JOIN_DONE.with(|flag| !*flag.borrow())
}

#[cfg(test)]
fn upsert_ai_user_message(ai: &mut AiState, msg: crate::state::ai::AiMessage) {
    frame_client_ai::upsert_ai_user_message(ai, msg);
}

#[cfg(test)]
fn pick_number(payload: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    frame_client_parse::pick_number(payload, keys)
}

#[cfg(test)]
fn pick_str<'a>(payload: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    frame_client_parse::pick_str(payload, keys)
}

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
    if should_record_trace(frame) {
        trace.update(|t| t.push_frame(frame.clone()));
    }

    if handle_session_connected_frame(frame, board, boards, tx) {
        return;
    }
    if handle_board_frame(frame, board, boards, tx) {
        return;
    }
    if queue_live_object_frame(board, frame) {
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
                        b.bump_scene_rev();
                        JOIN_ITEM_BUFFER.with(|buf| buf.borrow_mut().clear());
                    }
                });
                queue_join_item(board, obj);
            }
            true
        }
        Some("join") if frame.status == FrameStatus::Done => {
            flush_join_items(board);
            board.update(|b| {
                if let (Some(parent_id), Some(pending_id), Some(started_ms)) = (
                    frame.parent_id.as_deref(),
                    b.pending_join_request_id.as_deref(),
                    b.pending_join_started_ms,
                )
                    && parent_id == pending_id
                {
                    #[cfg(feature = "hydrate")]
                    {
                        b.join_round_trip_ms = Some((js_sys::Date::now() - started_ms).max(0.0));
                    }
                }
                b.pending_join_request_id = None;
                b.pending_join_started_ms = None;

                if let Some(objs) = parse_board_objects(&frame.data) {
                    b.objects.clear();
                    b.drag_objects.clear();
                    b.drag_updated_at.clear();
                    for obj in objs {
                        b.objects.insert(obj.id.clone(), obj);
                    }
                    b.bump_scene_rev();
                } else if !b.join_streaming {
                    // Empty stream: clear stale data from prior board snapshot.
                    b.objects.clear();
                    b.drag_objects.clear();
                    b.drag_updated_at.clear();
                    b.bump_scene_rev();
                }
                b.join_streaming = false;
                b.is_public = frame
                    .data
                    .get("is_public")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
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
                    frame_client_objects::upsert_presence_from_payload(b, &frame.data);
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
                        frame_client_objects::upsert_presence_from_payload(b, row);
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
                    if b.follow_client_id.as_deref() == Some(client_id) {
                        b.follow_client_id = None;
                    }
                    if b.jump_to_client_id.as_deref() == Some(client_id) {
                        b.jump_to_client_id = None;
                    }
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
        Some("visibility:set") if frame.status == FrameStatus::Done => {
            let is_public = frame
                .data
                .get("is_public")
                .and_then(serde_json::Value::as_bool);
            let board_id = frame
                .data
                .get("board_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or_else(|| frame.board_id.clone());
            if let (Some(board_id), Some(is_public)) = (board_id, is_public) {
                boards.update(|s| {
                    if let Some(item) = s.items.iter_mut().find(|item| item.id == board_id) {
                        item.is_public = is_public;
                    }
                });
                if board.get_untracked().board_id.as_deref() == Some(board_id.as_str()) {
                    board.update(|b| b.is_public = is_public);
                }
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
            b.is_public = false;
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
            b.join_round_trip_ms = None;
            b.bump_scene_rev();
        });
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/");
        }
    }
}

#[cfg(test)]
#[path = "frame_client_test.rs"]
mod frame_client_test;
