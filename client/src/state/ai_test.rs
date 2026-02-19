use super::*;

// =============================================================
// AiState defaults
// =============================================================

#[test]
fn ai_state_default_empty_messages() {
    let state = AiState::default();
    assert!(state.messages.is_empty());
}

#[test]
fn ai_state_default_not_loading() {
    let state = AiState::default();
    assert!(!state.loading);
}
