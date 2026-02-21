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
    /// Unique board identifier (UUID string).
    pub id: String,
    /// Display name of the board.
    pub name: String,
    /// Owner user ID (UUID string), if the board has an owner.
    #[serde(default)]
    pub owner_id: Option<String>,
    /// Whether the board is publicly accessible.
    #[serde(default)]
    pub is_public: bool,
    /// Lightweight geometry snapshot for the thumbnail preview.
    #[serde(default)]
    pub snapshot: Vec<BoardListPreviewObject>,
}

/// Lightweight object geometry for dashboard board previews.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BoardListPreviewObject {
    /// Shape or edge type (e.g. `"rect"`, `"arrow"`).
    pub kind: String,
    /// Left edge in world coordinates.
    pub x: f64,
    /// Top edge in world coordinates.
    pub y: f64,
    /// Bounding-box width in world coordinates.
    pub width: Option<f64>,
    /// Bounding-box height in world coordinates.
    pub height: Option<f64>,
    /// Clockwise rotation in degrees.
    pub rotation: f64,
    /// Stacking order index.
    pub z_index: i32,
}

/// Shared board list state backed by websocket frames.
#[derive(Clone, Debug, Default)]
pub struct BoardsState {
    /// Current list of boards visible to the authenticated user.
    pub items: Vec<BoardListItem>,
    /// Opaque revision token used for cache invalidation.
    pub list_rev: Option<String>,
    /// True while a `board:list` request is in flight.
    pub loading: bool,
    /// True while a `board:create` request is in flight.
    pub create_pending: bool,
    /// ID of a newly created board, used to redirect after creation.
    pub created_board_id: Option<String>,
    /// ID of a board joined via a redeemed access code.
    pub redeemed_board_id: Option<String>,
    /// Last error message from a board list or create operation.
    pub error: Option<String>,
}
