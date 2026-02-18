//! REST API helpers for communicating with the server.
//!
//! Client-side (hydrate): real HTTP calls via `gloo-net`.
//! Server-side (SSR): stubs returning `None`/error since these endpoints
//! are only meaningful in the browser.

#![allow(clippy::unused_async)]

#[cfg(feature = "hydrate")]
use super::types::{Frame, FrameStatus};
use super::types::{User, UserProfile};

/// Fetch the currently authenticated user from `/api/auth/me`.
/// Returns `None` if not authenticated or on the server.
pub async fn fetch_current_user() -> Option<User> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/auth/me")
            .send()
            .await
            .ok()?;
        if !resp.ok() {
            return None;
        }
        resp.json::<User>().await.ok()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        None
    }
}

/// Log out the current user by calling `POST /api/auth/logout`.
pub async fn logout() {
    #[cfg(feature = "hydrate")]
    {
        let _ = gloo_net::http::Request::post("/api/auth/logout")
            .send()
            .await;
    }
}

/// Fetch a user's profile from `/api/users/{user_id}/profile`.
pub async fn fetch_user_profile(user_id: &str) -> Option<UserProfile> {
    #[cfg(feature = "hydrate")]
    {
        let url = format!("/api/users/{user_id}/profile");
        let resp = gloo_net::http::Request::get(&url).send().await.ok()?;
        if !resp.ok() {
            return None;
        }
        resp.json::<UserProfile>().await.ok()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = user_id;
        None
    }
}

/// Create a WebSocket authentication ticket via `POST /api/auth/ws-ticket`.
///
/// # Errors
///
/// Returns an error string if the ticket request fails.
pub async fn create_ws_ticket() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/auth/ws-ticket")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(format!("ticket request failed: {}", resp.status()));
        }
        #[derive(serde::Deserialize)]
        struct TicketResponse {
            ticket: String,
        }
        let body: TicketResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body.ticket)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Err("not available on server".to_owned())
    }
}

/// Fetch the board list via websocket `board:list`.
pub async fn fetch_boards() -> Vec<BoardListItem> {
    #[cfg(feature = "hydrate")]
    {
        let Ok(frame) = ws_roundtrip("board:list", None, serde_json::json!({})).await else {
            return Vec::new();
        };

        frame
            .data
            .get("boards")
            .cloned()
            .and_then(|v| serde_json::from_value::<Vec<BoardListItem>>(v).ok())
            .unwrap_or_default()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Vec::new()
    }
}

/// Create a board via websocket `board:create`.
pub async fn create_board(name: &str) -> Option<BoardListItem> {
    #[cfg(feature = "hydrate")]
    {
        let frame = ws_roundtrip("board:create", None, serde_json::json!({ "name": name }))
            .await
            .ok()?;
        serde_json::from_value::<BoardListItem>(frame.data).ok()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = name;
        None
    }
}

/// A board summary for the dashboard list.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BoardListItem {
    pub id: String,
    pub name: String,
}

#[cfg(feature = "hydrate")]
async fn ws_roundtrip(syscall: &str, board_id: Option<String>, data: serde_json::Value) -> Result<Frame, String> {
    use futures::{SinkExt, StreamExt};
    use gloo_net::websocket::Message;
    use gloo_net::websocket::futures::WebSocket;

    let ticket = create_ws_ticket().await?;
    let location = web_sys::window()
        .and_then(|w| w.location().href().ok())
        .unwrap_or_default();
    let ws_proto = if location.starts_with("https") { "wss" } else { "ws" };
    let host = web_sys::window()
        .and_then(|w| w.location().host().ok())
        .unwrap_or_else(|| "localhost:3000".to_owned());
    let ws_url = format!("{ws_proto}://{host}/api/ws?ticket={ticket}");

    let ws = WebSocket::open(&ws_url).map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();

    let req = Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id,
        from: None,
        syscall: syscall.to_owned(),
        status: FrameStatus::Request,
        data,
    };

    let text = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    write.send(Message::Text(text)).await.map_err(|e| e.to_string())?;

    while let Some(msg) = read.next().await {
        let msg = msg.map_err(|e| e.to_string())?;
        if let Message::Text(text) = msg {
            let frame = serde_json::from_str::<Frame>(&text).map_err(|e| e.to_string())?;
            if frame.syscall == syscall {
                return match frame.status {
                    FrameStatus::Done => Ok(frame),
                    FrameStatus::Error => Err(frame
                        .data
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("request failed")
                        .to_owned()),
                    _ => continue,
                };
            }
        }
    }

    Err("websocket closed before response".to_owned())
}
