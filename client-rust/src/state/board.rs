#[cfg(test)]
#[path = "board_test.rs"]
mod board_test;

use std::collections::HashMap;

use crate::net::types::Presence;

/// Board-level state: which board is active, connection status, and presence.
///
/// In the full Leptos implementation, fields will be `RwSignal` types
/// provided via context. For now they are plain fields.
#[derive(Clone, Debug, Default)]
pub struct BoardState {
    pub board_id: Option<String>,
    pub board_name: Option<String>,
    pub connection_status: ConnectionStatus,
    pub presence: HashMap<String, Presence>,
}

/// WebSocket connection status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}
