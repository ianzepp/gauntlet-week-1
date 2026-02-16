//! Cursor service — ephemeral cursor position broadcast.
//!
//! DESIGN
//! ======
//! Cursor positions are purely ephemeral: broadcast to board peers and
//! immediately forgotten. No persistence, no state storage.

use uuid::Uuid;

use crate::frame::{Data, Frame};
use crate::state::AppState;

/// Broadcast a cursor position to all board peers except the sender.
pub async fn broadcast_cursor(state: &AppState, board_id: Uuid, from_client_id: Uuid, x: f64, y: f64, from_name: &str) {
    let mut data = Data::new();
    data.insert("client_id".into(), serde_json::json!(from_client_id));
    data.insert("x".into(), serde_json::json!(x));
    data.insert("y".into(), serde_json::json!(y));
    data.insert("name".into(), serde_json::json!(from_name));

    let frame = Frame::request("cursor:moved", data).with_board_id(board_id);

    crate::services::board::broadcast(state, board_id, &frame, Some(from_client_id)).await;
}
