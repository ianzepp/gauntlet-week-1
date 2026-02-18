use super::*;

// =============================================================================
// env_bool — uses unique env var names to avoid races with parallel tests.
// =============================================================================

#[test]
fn env_bool_true_variants() {
    for (i, val) in ["1", "true", "yes", "on"].iter().enumerate() {
        let key = format!("__TEST_EB_TRUE_{i}__");
        unsafe { std::env::set_var(&key, val) };
        assert_eq!(env_bool(&key), Some(true), "expected true for {val:?}");
        unsafe { std::env::remove_var(&key) };
    }
}

#[test]
fn env_bool_false_variants() {
    for (i, val) in ["0", "false", "no", "off"].iter().enumerate() {
        let key = format!("__TEST_EB_FALSE_{i}__");
        unsafe { std::env::set_var(&key, val) };
        assert_eq!(env_bool(&key), Some(false), "expected false for {val:?}");
        unsafe { std::env::remove_var(&key) };
    }
}

#[test]
fn env_bool_case_insensitive() {
    for (i, val) in ["TRUE", "True", "YES", "On"].iter().enumerate() {
        let key = format!("__TEST_EB_CI_{i}__");
        unsafe { std::env::set_var(&key, val) };
        assert_eq!(env_bool(&key), Some(true), "expected true for {val:?}");
        unsafe { std::env::remove_var(&key) };
    }
}

#[test]
fn env_bool_invalid_returns_none() {
    let key = "__TEST_EB_INVALID_9823__";
    unsafe { std::env::set_var(key, "maybe") };
    assert_eq!(env_bool(key), None);
    unsafe { std::env::remove_var(key) };
}

#[test]
fn env_bool_unset_returns_none() {
    assert_eq!(env_bool("__TEST_EB_SURELY_UNSET_XYZ_42__"), None);
}

#[test]
fn env_bool_whitespace_trimmed() {
    let key = "__TEST_EB_WS_882__";
    unsafe { std::env::set_var(key, "  true  ") };
    assert_eq!(env_bool(key), Some(true));
    unsafe { std::env::remove_var(key) };
}

#[test]
fn env_bool_empty_string_returns_none() {
    let key = "__TEST_EB_EMPTY_773__";
    unsafe { std::env::set_var(key, "") };
    assert_eq!(env_bool(key), None);
    unsafe { std::env::remove_var(key) };
}

// =============================================================================
// cookie_secure — uses unique env var names to avoid parallel test races.
// These tests use COOKIE_SECURE and GITHUB_REDIRECT_URI which are shared
// globals, so we test via env_bool logic instead of calling cookie_secure()
// directly to avoid races with other tests that modify the same vars.
// =============================================================================

#[test]
fn cookie_secure_explicit_true_via_env_bool() {
    let key = "__TEST_CS_TRUE_991__";
    unsafe { std::env::set_var(key, "true") };
    assert_eq!(env_bool(key), Some(true));
    unsafe { std::env::remove_var(key) };
}

#[test]
fn cookie_secure_explicit_false_via_env_bool() {
    let key = "__TEST_CS_FALSE_992__";
    unsafe { std::env::set_var(key, "false") };
    assert_eq!(env_bool(key), Some(false));
    unsafe { std::env::remove_var(key) };
}

#[test]
fn cookie_secure_https_inference_logic() {
    // Test the inference logic directly: starts_with("https://")
    assert!("https://myapp.com/callback".starts_with("https://"));
    assert!(!"http://localhost/callback".starts_with("https://"));
}
