use super::*;

#[test]
fn normalize_email_accepts_basic_address() {
    assert_eq!(normalize_email("  USER@Example.com "), Some("user@example.com".to_owned()));
}

#[test]
fn normalize_email_rejects_invalid_values() {
    assert_eq!(normalize_email(""), None);
    assert_eq!(normalize_email("user"), None);
    assert_eq!(normalize_email("@example.com"), None);
    assert_eq!(normalize_email("user@"), None);
    assert_eq!(normalize_email("a@b@c"), None);
}

#[test]
fn normalize_code_accepts_upper_and_normalizes() {
    let code = generate_access_code();
    assert_eq!(normalize_code(&code), Some(code.clone()));
    assert_eq!(normalize_code("abc234"), Some("ABC234".to_owned()));
}

#[test]
fn normalize_code_rejects_bad_shapes() {
    assert_eq!(normalize_code("abc12"), None);
    assert_eq!(normalize_code("abc1234"), None);
    assert_eq!(normalize_code("ABC1I0"), None);
    assert_eq!(normalize_code("ABC12!"), None);
}

#[test]
fn generate_access_code_shape() {
    let code = generate_access_code();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| CODE_ALPHABET.contains(&(c as u8))));
}

#[test]
fn hash_access_code_is_stable() {
    let a = hash_access_code("ABC123");
    let b = hash_access_code("ABC123");
    let c = hash_access_code("ABC124");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn render_template_injects_email_and_code() {
    let html = render_email_auth_template("user@example.com", "ABC234");
    assert!(html.contains("user@example.com"));
    assert!(html.contains("ABC234"));
    assert!(!html.contains("{{EMAIL}}"));
    assert!(!html.contains("{{CODE}}"));
}
