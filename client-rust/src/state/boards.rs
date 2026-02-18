#[cfg(test)]
#[path = "boards_test.rs"]
mod boards_test;

/// A board summary for dashboard/mission-control lists.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BoardListItem {
    pub id: String,
    pub name: String,
}

/// Shared board list state backed by websocket frames.
#[derive(Clone, Debug, Default)]
pub struct BoardsState {
    pub items: Vec<BoardListItem>,
    pub loading: bool,
    pub create_pending: bool,
    pub created_board_id: Option<String>,
}
