use super::*;
use crate::state::boards::BoardsState;

#[test]
fn apply_board_error_state_handles_board_list() {
    let mut boards = BoardsState { loading: true, ..BoardsState::default() };
    let handled = apply_board_error_state("board:list", "failed", &mut boards);
    assert!(handled);
    assert!(!boards.loading);
    assert_eq!(boards.error.as_deref(), Some("failed"));
}

#[test]
fn apply_board_error_state_handles_board_create() {
    let mut boards = BoardsState { create_pending: true, ..BoardsState::default() };
    let handled = apply_board_error_state("board:create", "nope", &mut boards);
    assert!(handled);
    assert!(!boards.create_pending);
    assert_eq!(boards.error.as_deref(), Some("nope"));
}

#[test]
fn apply_board_error_state_returns_false_for_unrelated_syscall() {
    let mut boards = BoardsState { loading: true, create_pending: true, ..BoardsState::default() };
    let handled = apply_board_error_state("chat:message", "ignored", &mut boards);
    assert!(!handled);
    assert!(boards.loading);
    assert!(boards.create_pending);
    assert!(boards.error.is_none());
}
