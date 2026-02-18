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
use crate::state::auth::AuthState;
#[cfg(feature = "hydrate")]
use crate::state::board::{BoardState, ConnectionStatus};
#[cfg(feature = "hydrate")]
use crate::state::chat::{ChatMessage, ChatState};

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
    board: leptos::prelude::RwSignal<BoardState>,
    chat: leptos::prelude::RwSignal<ChatState>,
) -> futures::channel::mpsc::UnboundedSender<String> {
    use futures::channel::mpsc;

    let (tx, rx) = mpsc::unbounded::<String>();
    let tx_clone = tx.clone();

    leptos::task::spawn_local(frame_client_loop(auth, board, chat, tx_clone, rx));

    tx
}

/// Main connection loop with reconnect logic.
#[cfg(feature = "hydrate")]
async fn frame_client_loop(
    auth: leptos::prelude::RwSignal<AuthState>,
    board: leptos::prelude::RwSignal<BoardState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    tx: futures::channel::mpsc::UnboundedSender<String>,
    rx: futures::channel::mpsc::UnboundedReceiver<String>,
) {
    use futures::StreamExt;
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

        match connect_and_run(&ws_url, auth, board, chat, &tx, &rx).await {
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
    board: leptos::prelude::RwSignal<BoardState>,
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
                        dispatch_frame(&frame, auth, board, chat, tx);
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
    board: leptos::prelude::RwSignal<BoardState>,
    chat: leptos::prelude::RwSignal<ChatState>,
    _tx: &futures::channel::mpsc::UnboundedSender<String>,
) {
    use crate::net::types::{BoardObject, FrameStatus, Presence};

    let syscall = frame.syscall.as_str();

    match syscall {
        "session:connected" => {
            board.update(|b| b.connection_status = ConnectionStatus::Connected);
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
            if let Ok(msg) = serde_json::from_value::<ChatMessage>(frame.data.clone()) {
                chat.update(|c| c.messages.push(msg));
            }
        }

        _ => {}
    }
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
