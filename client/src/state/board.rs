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
    pub board_id: Option<String>,
    pub board_name: Option<String>,
    pub is_public: bool,
    pub self_client_id: Option<String>,
    pub follow_client_id: Option<String>,
    pub jump_to_client_id: Option<String>,
    pub connection_status: ConnectionStatus,
    pub presence: HashMap<String, Presence>,
    pub cursor_updated_at: HashMap<String, i64>,
    pub objects: HashMap<String, BoardObject>,
    pub savepoints: Vec<Savepoint>,
    pub selection: HashSet<String>,
    pub drag_objects: HashMap<String, BoardObject>,
    pub drag_updated_at: HashMap<String, i64>,
    pub join_streaming: bool,
    pub generated_access_code: Option<String>,
}

/// WebSocket connection status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}
