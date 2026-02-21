use super::*;

#[test]
fn user_profile_endpoint_formats_expected_path() {
    assert_eq!(user_profile_endpoint("u123"), "/api/users/u123/profile");
}

#[test]
fn ticket_request_failed_message_formats_status() {
    assert_eq!(ticket_request_failed_message(401), "ticket request failed: 401");
}

#[test]
fn request_code_failed_message_formats_status() {
    assert_eq!(request_code_failed_message(429), "request code failed: 429");
}

#[test]
fn verify_code_failed_message_formats_status() {
    assert_eq!(verify_code_failed_message(400), "verify code failed: 400");
}
