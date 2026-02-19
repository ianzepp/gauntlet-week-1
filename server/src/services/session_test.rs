use super::*;

// =============================================================================
// bytes_to_hex
// =============================================================================

#[test]
fn bytes_to_hex_empty() {
    assert_eq!(bytes_to_hex(&[]), "");
}

#[test]
fn bytes_to_hex_single_byte() {
    assert_eq!(bytes_to_hex(&[0xff]), "ff");
}

#[test]
fn bytes_to_hex_leading_zero() {
    assert_eq!(bytes_to_hex(&[0x0a]), "0a");
}

#[test]
fn bytes_to_hex_multi_byte() {
    assert_eq!(bytes_to_hex(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
}

#[test]
fn bytes_to_hex_all_zeros() {
    assert_eq!(bytes_to_hex(&[0x00, 0x00, 0x00]), "000000");
}

// =============================================================================
// generate_token
// =============================================================================

#[test]
fn generate_token_is_64_hex_chars() {
    let token = generate_token();
    assert_eq!(token.len(), 64);
}

#[test]
fn generate_token_all_valid_hex() {
    let token = generate_token();
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn generate_token_two_calls_differ() {
    let a = generate_token();
    let b = generate_token();
    assert_ne!(a, b);
}

// =============================================================================
// generate_ws_ticket
// =============================================================================

#[test]
fn generate_ws_ticket_is_32_hex_chars() {
    let ticket = generate_ws_ticket();
    assert_eq!(ticket.len(), 32);
}

#[test]
fn generate_ws_ticket_all_valid_hex() {
    let ticket = generate_ws_ticket();
    assert!(ticket.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn generate_ws_ticket_two_calls_differ() {
    let a = generate_ws_ticket();
    let b = generate_ws_ticket();
    assert_ne!(a, b);
}

// =============================================================================
// SessionUser
// =============================================================================

#[test]
fn session_user_debug() {
    let user = SessionUser {
        id: Uuid::nil(),
        name: "alice".into(),
        avatar_url: None,
        color: "#FF0000".into(),
        auth_method: "github".into(),
    };
    let debug = format!("{user:?}");
    assert!(debug.contains("alice"));
}

#[test]
fn session_user_clone() {
    let user = SessionUser {
        id: Uuid::nil(),
        name: "bob".into(),
        avatar_url: Some("https://example.com/avatar.png".into()),
        color: "#00FF00".into(),
        auth_method: "email".into(),
    };
    let cloned = user.clone();
    assert_eq!(cloned.id, user.id);
    assert_eq!(cloned.name, user.name);
    assert_eq!(cloned.avatar_url, user.avatar_url);
    assert_eq!(cloned.color, user.color);
}

#[test]
fn session_user_serialize_round_trip() {
    let user = SessionUser {
        id: Uuid::nil(),
        name: "charlie".into(),
        avatar_url: Some("https://example.com/pic.png".into()),
        color: "#0000FF".into(),
        auth_method: "session".into(),
    };
    let json = serde_json::to_string(&user).unwrap();
    let restored: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(restored["name"], "charlie");
    assert_eq!(restored["color"], "#0000FF");
    assert_eq!(restored["avatar_url"], "https://example.com/pic.png");
}

#[test]
fn session_user_serialize_none_avatar() {
    let user = SessionUser {
        id: Uuid::nil(),
        name: "dave".into(),
        avatar_url: None,
        color: "#FFFFFF".into(),
        auth_method: "email".into(),
    };
    let json = serde_json::to_string(&user).unwrap();
    let restored: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(restored["avatar_url"].is_null());
}
