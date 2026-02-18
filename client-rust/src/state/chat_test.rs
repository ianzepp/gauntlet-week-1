use super::*;

// =============================================================
// ChatState defaults
// =============================================================

#[test]
fn chat_state_default_empty_messages() {
    let state = ChatState::default();
    assert!(state.messages.is_empty());
}
