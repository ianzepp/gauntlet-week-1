//! Board-session state for the active workspace.
//!
//! SYSTEM CONTEXT
//! ==============
//! This model stores the local projection of one joined board, including
//! real-time objects, peer presence, and transient interaction overlays.

#[cfg(test)]
#[path = "board_test.rs"]
mod board_test;

use std::collections::{HashMap, HashSet};

use crate::net::types::{BoardObject, Presence, Savepoint};

/// Board-level state: which board is active, connection status, objects, and presence.
#[derive(Clone, Debug, Default)]
pub struct BoardState {
    /// ID of the currently active board (UUID string).
    pub board_id: Option<String>,
    /// Display name of the active board.
    pub board_name: Option<String>,
    /// Whether the board is publicly accessible.
    pub is_public: bool,
    /// WebSocket client ID assigned to this local session.
    pub self_client_id: Option<String>,
    /// Client ID being followed in camera-follow mode.
    pub follow_client_id: Option<String>,
    /// Client ID to jump the camera to on the next render.
    pub jump_to_client_id: Option<String>,
    /// Current WebSocket connection lifecycle state.
    pub connection_status: ConnectionStatus,
    /// Live presence data keyed by client ID.
    pub presence: HashMap<String, Presence>,
    /// Timestamp of the last cursor update received per client ID.
    pub cursor_updated_at: HashMap<String, i64>,
    /// All board objects keyed by object ID.
    pub objects: HashMap<String, BoardObject>,
    /// Board savepoints available for rewind.
    pub savepoints: Vec<Savepoint>,
    /// Currently selected object IDs.
    pub selection: HashSet<String>,
    /// Optimistic object positions during an in-progress drag, keyed by object ID.
    pub drag_objects: HashMap<String, BoardObject>,
    /// Timestamp of the last drag position update per object ID.
    pub drag_updated_at: HashMap<String, i64>,
    /// True while the initial `board:join` object stream is still in flight.
    pub join_streaming: bool,
    /// Access code generated for sharing, if any.
    pub generated_access_code: Option<String>,
    /// Most recent board:join round-trip latency in milliseconds.
    pub join_round_trip_ms: Option<f64>,
    /// Outbound board:join request ID awaiting completion.
    pub pending_join_request_id: Option<String>,
    /// Client-side timestamp (ms) when pending board:join was sent.
    pub pending_join_started_ms: Option<f64>,
}

/// WebSocket connection status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Not connected; socket is closed or not yet opened.
    #[default]
    Disconnected,
    /// WebSocket handshake is in progress.
    Connecting,
    /// WebSocket is open and the server sent `session:connected`.
    Connected,
}
