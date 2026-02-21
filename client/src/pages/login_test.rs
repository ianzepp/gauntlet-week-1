use super::*;

#[test]
fn normalize_code_input_uppercases_letters() {
    assert_eq!(normalize_code_input("abc123"), "ABC123");
}

#[test]
fn validate_request_code_input_trims_and_requires_value() {
    assert_eq!(
        validate_request_code_input("  user@example.com  "),
        Ok("user@example.com".to_owned())
    );
    assert_eq!(validate_request_code_input("   "), Err("Enter an email first."));
}

#[test]
fn validate_verify_code_input_trims_and_requires_both_fields() {
    assert_eq!(
        validate_verify_code_input(" a@b.com ", " abc123 "),
        Ok(("a@b.com".to_owned(), "abc123".to_owned()))
    );
    assert_eq!(
        validate_verify_code_input("", "abc123"),
        Err("Enter both email and 6-char code.")
    );
    assert_eq!(
        validate_verify_code_input("a@b.com", "   "),
        Err("Enter both email and 6-char code.")
    );
}

#[test]
fn validate_verify_code_input_code_too_short() {
    assert_eq!(
        validate_verify_code_input("a@b.com", "ABCDE"),
        Err("Enter both email and 6-char code.")
    );
}

#[test]
fn validate_verify_code_input_code_too_long() {
    assert_eq!(
        validate_verify_code_input("a@b.com", "ABCDEFG"),
        Err("Enter both email and 6-char code.")
    );
}
