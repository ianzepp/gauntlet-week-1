#[cfg(test)]
#[path = "board_test.rs"]
mod board_test;

use std::collections::{HashMap, HashSet};

use crate::net::types::{BoardObject, Presence};

/// Board-level state: which board is active, connection status, objects, and presence.
#[derive(Clone, Debug, Default)]
pub struct BoardState {
    pub board_id: Option<String>,
    pub board_name: Option<String>,
    pub connection_status: ConnectionStatus,
    pub presence: HashMap<String, Presence>,
    pub cursor_updated_at: HashMap<String, i64>,
    pub objects: HashMap<String, BoardObject>,
    pub selection: HashSet<String>,
    pub drag_objects: HashMap<String, BoardObject>,
    pub drag_updated_at: HashMap<String, i64>,
}

/// WebSocket connection status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}
