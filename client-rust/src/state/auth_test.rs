use super::*;

// =============================================================
// AuthState defaults
// =============================================================

#[test]
fn auth_state_default_no_user() {
    let state = AuthState::default();
    assert!(state.user.is_none());
}

#[test]
fn auth_state_default_not_loading() {
    let state = AuthState::default();
    assert!(!state.loading);
}
