
use super::*;

#[test]
fn per_client_allows_up_to_limit() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    for i in 0..PER_CLIENT_LIMIT {
        assert!(rl.check_and_record_at(client, now).is_ok(), "request {i} should succeed");
    }
    assert!(matches!(
        rl.check_and_record_at(client, now),
        Err(RateLimitError::PerClientExceeded)
    ));
}

#[test]
fn global_allows_up_to_limit() {
    let rl = RateLimiter::new();
    let now = Instant::now();

    // Use distinct clients to avoid hitting per-client limit first.
    for i in 0..GLOBAL_LIMIT {
        let client = Uuid::new_v4();
        assert!(rl.check_and_record_at(client, now).is_ok(), "request {i} should succeed");
    }
    let client = Uuid::new_v4();
    assert!(matches!(
        rl.check_and_record_at(client, now),
        Err(RateLimitError::GlobalExceeded)
    ));
}

#[test]
fn token_budget_exceeded() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    rl.record_tokens_at(client, TOKEN_BUDGET, now);

    assert!(matches!(
        rl.check_token_budget_at(client, now),
        Err(RateLimitError::TokenBudgetExceeded)
    ));
}

#[test]
fn window_expiry_allows_new_requests() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let start = Instant::now();

    // Fill up per-client limit.
    for _ in 0..PER_CLIENT_LIMIT {
        rl.check_and_record_at(client, start).unwrap();
    }
    assert!(rl.check_and_record_at(client, start).is_err());

    // After the window passes, requests should succeed again.
    let after_window = start + PER_CLIENT_WINDOW + Duration::from_millis(1);
    assert!(rl.check_and_record_at(client, after_window).is_ok());
}

#[test]
fn distinct_clients_do_not_interfere() {
    let rl = RateLimiter::new();
    let client_a = Uuid::new_v4();
    let client_b = Uuid::new_v4();
    let now = Instant::now();

    // Fill up client A.
    for _ in 0..PER_CLIENT_LIMIT {
        rl.check_and_record_at(client_a, now).unwrap();
    }
    assert!(rl.check_and_record_at(client_a, now).is_err());

    // Client B should still be able to make requests.
    assert!(rl.check_and_record_at(client_b, now).is_ok());
}
