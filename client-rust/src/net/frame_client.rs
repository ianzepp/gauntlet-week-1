//! WebSocket frame client for real-time communication with the server.
//!
//! The `FrameClient` manages the WebSocket lifecycle: connection, reconnection
//! with exponential backoff, frame dispatch, and signal updates. It is the
//! primary bridge between the server's frame protocol and the Leptos UI state.
//!
//! All WebSocket logic is gated behind `#[cfg(feature = "hydrate")]` since it
//! requires a browser environment.

#[cfg(feature = "hydrate")]
use crate::net::types::Frame;
#[cfg(feature = "hydrate")]
use crate::state::ai::{AiMessage, AiState};
#[cfg(feature = "hydrate")]
use crate::state::auth::AuthState;
#[cfg(feature = "hydrate")]
use crate::state::board::{BoardState, ConnectionStatus};
#[cfg(feature = "hydrate")]
use crate::state::boards::{BoardListItem, BoardsState};
#[cfg(feature = "hydrate")]
use crate::state::chat::{ChatMessage, ChatState};
#[cfg(feature = "hydrate")]
use leptos::prelude::GetUntracked;
#[cfg(feature = "hydrate")]
use leptos::prelude::Update;

/// Send a frame to the server via the shared sender channel.
///
/// Returns `false` if the channel is closed (no active connection).
#[cfg(feature = "hydrate")]
pub fn send_frame(tx: &futures::channel::mpsc::UnboundedSender<String>, frame: &Frame) -> bool {
    if let Ok(json) = serde_json::to_string(frame) {
        tx.unbounded_send(json).is_ok()
    } else {
        false
    }
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
) -> futures::channel::mpsc::UnboundedSender<String> {
    use futures::channel::mpsc;

    let (tx, rx) = mpsc::unbounded::<String>();
    let tx_clone = tx.clone();

    leptos::task::spawn_local(frame_client_loop(auth, ai, board, boards, chat, tx_clone, rx));

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
    tx: futures::channel::mpsc::UnboundedSender<String>,
    rx: futures::channel::mpsc::UnboundedReceiver<String>,
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

        match connect_and_run(&ws_url, auth, ai, board, boards, chat, &tx, &rx).await {
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
    tx: &futures::channel::mpsc::UnboundedSender<String>,
    rx: &std::rc::Rc<std::cell::RefCell<futures::channel::mpsc::UnboundedReceiver<String>>>,
) -> Result<(), String> {
    use futures::StreamExt;
    use gloo_net::websocket::Message;
    use gloo_net::websocket::futures::WebSocket;

    let ws = WebSocket::open(url).map_err(|e| e.to_string())?;
    let (mut ws_write, mut ws_read) = ws.split();

    board.update(|b| b.connection_status = ConnectionStatus::Connected);
    send_board_join_for_active_board(tx, board);

    // Spawn a task to forward outgoing messages from our channel to the WS.
    let mut rx_borrow = rx.borrow_mut();
    let send_task = async {
        use futures::SinkExt;
        while let Some(msg) = rx_borrow.next().await {
            if ws_write.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    };

    // Receive loop: process incoming frames.
    let recv_task = async {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(frame) = serde_json::from_str::<Frame>(&text) {
                        dispatch_frame(&frame, auth, ai, board, boards, chat, tx);
                    }
                }
                Ok(Message::Bytes(_)) => {}
                Err(e) => {
                    leptos::logging::warn!("WS recv error: {e}");
                    break;
                }
            }
        }
    };

    // Run both tasks; when either finishes, the connection is done.
    futures::future::select(Box::pin(send_task), Box::pin(recv_task)).await;

    Ok(())
}

/// Dispatch an incoming frame to the appropriate state handler.
#[cfg(feature = "hydrate")]
fn dispatch_frame(
    frame: &Frame,
    _auth: leptos::prelude::RwSignal<AuthState>,
    ai: leptos::prelude::RwSignal<AiState>,
    board: leptos::prelude::RwSignal<BoardState>,
    boards: leptos::prelude::RwSignal<BoardsState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    tx: &futures::channel::mpsc::UnboundedSender<String>,
) {
    use crate::net::types::{BoardObject, FrameStatus, Presence};

    let syscall = frame.syscall.as_str();

    match syscall {
        "session:connected" => {
            board.update(|b| b.connection_status = ConnectionStatus::Connected);
            send_board_join_for_active_board(tx, board);
            send_board_list_request(tx);
        }

        "board:join" if frame.status == FrameStatus::Done => {
            // Load objects snapshot from the join response.
            if let Some(objects) = frame.data.get("objects") {
                if let Ok(objs) = serde_json::from_value::<Vec<BoardObject>>(objects.clone()) {
                    board.update(|b| {
                        b.objects.clear();
                        for obj in objs {
                            b.objects.insert(obj.id.clone(), obj);
                        }
                    });
                }
            }
            // Set board name if present.
            if let Some(name) = frame.data.get("name").and_then(|n| n.as_str()) {
                board.update(|b| b.board_name = Some(name.to_owned()));
            }
        }

        "board:list" if frame.status == FrameStatus::Done => {
            let list = frame
                .data
                .get("boards")
                .cloned()
                .and_then(|v| serde_json::from_value::<Vec<BoardListItem>>(v).ok())
                .unwrap_or_default();
            boards.update(|s| {
                s.items = list;
                s.loading = false;
            });
        }

        "board:create" if frame.status == FrameStatus::Done => {
            if let Ok(created) = serde_json::from_value::<BoardListItem>(frame.data.clone()) {
                boards.update(|s| {
                    if let Some(existing) = s.items.iter_mut().find(|b| b.id == created.id) {
                        *existing = created.clone();
                    } else {
                        s.items.insert(0, created.clone());
                    }
                    s.create_pending = false;
                    s.created_board_id = Some(created.id.clone());
                });
                send_board_list_request(tx);
            } else {
                boards.update(|s| s.create_pending = false);
            }
        }

        "board:join" => {
            // Peer join broadcast may include only client/user identifiers.
            if frame.data.get("client_id").is_some() && frame.data.get("objects").is_none() {
                if let Some(user_id) = frame.data.get("user_id").and_then(|v| v.as_str()) {
                    board.update(|b| {
                        b.presence.entry(user_id.to_owned()).or_insert(Presence {
                            user_id: user_id.to_owned(),
                            name: "Agent".to_owned(),
                            color: "#8a8178".to_owned(),
                            cursor: None,
                        });
                    });
                }
            }
        }

        "object:create" if frame.status == FrameStatus::Done => {
            if let Ok(obj) = serde_json::from_value::<BoardObject>(frame.data.clone()) {
                board.update(|b| {
                    b.objects.insert(obj.id.clone(), obj);
                });
            }
        }

        "object:update" if frame.status == FrameStatus::Done => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                board.update(|b| {
                    if let Some(existing) = b.objects.get_mut(id) {
                        merge_object_update(existing, &frame.data);
                    }
                });
            }
        }

        "object:delete" if frame.status == FrameStatus::Done => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                board.update(|b| {
                    b.objects.remove(id);
                    b.selection.remove(id);
                });
            }
        }

        "cursor:moved" => {
            if let Ok(p) = serde_json::from_value::<Presence>(frame.data.clone()) {
                board.update(|b| {
                    b.presence.insert(p.user_id.clone(), p);
                });
            }
        }

        "board:part" => {
            if let Some(user_id) = frame.data.get("user_id").and_then(|v| v.as_str()) {
                board.update(|b| {
                    b.presence.remove(user_id);
                });
            }
        }

        "chat:message" if frame.status == FrameStatus::Done => {
            if let Some(msg) = parse_chat_message(frame, &frame.data) {
                chat.update(|c| c.messages.push(msg));
            }
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
        }

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
        }

        "ai:prompt" if frame.status == FrameStatus::Done || frame.status == FrameStatus::Error => {
            if let Some(msg) = parse_ai_prompt_message(frame) {
                ai.update(|a| {
                    a.messages.push(msg);
                    a.loading = false;
                });
            } else if frame.status == FrameStatus::Error {
                let content = frame
                    .data
                    .get("error")
                    .and_then(|v| v.as_str())
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
        }

        _ if frame.status == FrameStatus::Error => {
            if frame.syscall == "board:list" {
                boards.update(|s| s.loading = false);
            } else if frame.syscall == "board:create" {
                boards.update(|s| s.create_pending = false);
            }
            leptos::logging::warn!("frame error: syscall={} data={}", frame.syscall, frame.data);
        }

        "gateway:error" => {
            leptos::logging::warn!("gateway:error frame: {}", frame.data);
        }

        _ => {}
    }
}

#[cfg(feature = "hydrate")]
fn send_board_join_for_active_board(
    tx: &futures::channel::mpsc::UnboundedSender<String>,
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
        syscall: "board:join".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    };
    let _ = send_frame(tx, &frame);
}

#[cfg(feature = "hydrate")]
fn send_board_list_request(tx: &futures::channel::mpsc::UnboundedSender<String>) {
    let frame = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: None,
        from: None,
        syscall: "board:list".to_owned(),
        status: crate::net::types::FrameStatus::Request,
        data: serde_json::json!({}),
    };
    let _ = send_frame(tx, &frame);
}

/// Merge partial object updates into an existing `BoardObject`.
#[cfg(feature = "hydrate")]
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
    if let Some(z) = data.get("z_index").and_then(|v| v.as_i64()) {
        #[allow(clippy::cast_possible_truncation)]
        {
            obj.z_index = z as i32;
        }
    }
    if let Some(props) = data.get("props") {
        obj.props = props.clone();
    }
    if let Some(v) = data.get("version").and_then(|v| v.as_i64()) {
        obj.version = v;
    }
}

#[cfg(feature = "hydrate")]
fn parse_chat_message(frame: &Frame, data: &serde_json::Value) -> Option<ChatMessage> {
    let content = data
        .get("content")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("message").and_then(|v| v.as_str()))?
        .to_owned();

    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| frame.id.clone());

    let user_id = data
        .get("user_id")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("from").and_then(|v| v.as_str()))
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

    let timestamp = data
        .get("timestamp")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        .or_else(|| {
            data.get("ts")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        })
        .unwrap_or(frame.ts as f64);

    Some(ChatMessage { id, user_id, user_name, user_color, content, timestamp })
}

#[cfg(feature = "hydrate")]
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
    let content = data
        .get("content")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("text").and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_owned();
    if content.trim().is_empty() {
        return None;
    }
    let timestamp = data
        .get("timestamp")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        .or_else(|| {
            data.get("ts")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        })
        .unwrap_or(0.0);
    let mutations = data.get("mutations").and_then(|v| v.as_i64());

    Some(AiMessage { id, role, content, timestamp, mutations })
}

#[cfg(feature = "hydrate")]
fn parse_ai_prompt_message(frame: &Frame) -> Option<AiMessage> {
    if let Some(msg) = parse_ai_message_value(&frame.data) {
        return Some(msg);
    }

    let content = frame
        .data
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| frame.data.get("content").and_then(|v| v.as_str()))?;

    Some(AiMessage {
        id: frame.id.clone(),
        role: if frame.status == crate::net::types::FrameStatus::Error {
            "error".to_owned()
        } else {
            "assistant".to_owned()
        },
        content: content.to_owned(),
        timestamp: frame.ts as f64,
        mutations: frame.data.get("mutations").and_then(|v| v.as_i64()),
    })
}
