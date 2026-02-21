use super::*;
use crate::net::types::User;

#[test]
fn should_redirect_unauth_when_not_loading_and_user_missing() {
    let state = AuthState { user: None, loading: false };
    assert!(should_redirect_unauth(&state));
}

#[test]
fn should_not_redirect_while_loading() {
    let state = AuthState { user: None, loading: true };
    assert!(!should_redirect_unauth(&state));
}

#[test]
fn should_not_redirect_when_user_exists() {
    let state = AuthState {
        user: Some(User {
            id: "u1".to_owned(),
            name: "Alice".to_owned(),
            avatar_url: None,
            color: "#123456".to_owned(),
            auth_method: "session".to_owned(),
        }),
        loading: false,
    };
    assert!(!should_redirect_unauth(&state));
}
