use super::*;

// =============================================================
// ConnectionStatus
// =============================================================

#[test]
fn connection_status_default_is_disconnected() {
    assert_eq!(ConnectionStatus::default(), ConnectionStatus::Disconnected);
}

#[test]
fn connection_status_variants_are_distinct() {
    assert_ne!(ConnectionStatus::Disconnected, ConnectionStatus::Connecting);
    assert_ne!(ConnectionStatus::Disconnected, ConnectionStatus::Connected);
    assert_ne!(ConnectionStatus::Connecting, ConnectionStatus::Connected);
}

// =============================================================
// BoardState defaults
// =============================================================

#[test]
fn board_state_default_no_board() {
    let state = BoardState::default();
    assert!(state.board_id.is_none());
    assert!(state.board_name.is_none());
}

#[test]
fn board_state_default_disconnected() {
    let state = BoardState::default();
    assert_eq!(state.connection_status, ConnectionStatus::Disconnected);
}

#[test]
fn board_state_default_empty_presence() {
    let state = BoardState::default();
    assert!(state.presence.is_empty());
}

#[test]
fn board_state_default_empty_objects() {
    let state = BoardState::default();
    assert!(state.objects.is_empty());
}

#[test]
fn board_state_default_empty_selection() {
    let state = BoardState::default();
    assert!(state.selection.is_empty());
}

#[test]
fn board_state_default_follow_and_jump_unset() {
    let state = BoardState::default();
    assert!(state.follow_client_id.is_none());
    assert!(state.jump_to_client_id.is_none());
    assert!(state.self_client_id.is_none());
}

#[test]
fn board_state_default_transient_maps_are_empty() {
    let state = BoardState::default();
    assert!(state.cursor_updated_at.is_empty());
    assert!(state.drag_objects.is_empty());
    assert!(state.drag_updated_at.is_empty());
    assert!(state.savepoints.is_empty());
    assert!(!state.join_streaming);
}
