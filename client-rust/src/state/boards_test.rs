use super::*;

#[test]
fn boards_state_defaults() {
    let s = BoardsState::default();
    assert!(s.items.is_empty());
    assert!(!s.loading);
    assert!(!s.create_pending);
    assert!(s.created_board_id.is_none());
}
