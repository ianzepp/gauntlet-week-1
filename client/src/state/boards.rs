//! Board-list state for dashboard and mission-control views.
//!
//! DESIGN
//! ======
//! Separating list state from active-board state avoids accidental coupling
//! between navigation inventory and in-board editing data.

#[cfg(test)]
#[path = "boards_test.rs"]
mod boards_test;

/// A board summary for dashboard/mission-control lists.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BoardListItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub snapshot: Vec<BoardListPreviewObject>,
}

/// Lightweight object geometry for dashboard board previews.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BoardListPreviewObject {
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: f64,
    pub z_index: i32,
}

/// Shared board list state backed by websocket frames.
#[derive(Clone, Debug, Default)]
pub struct BoardsState {
    pub items: Vec<BoardListItem>,
    pub loading: bool,
    pub create_pending: bool,
    pub created_board_id: Option<String>,
    pub error: Option<String>,
}
