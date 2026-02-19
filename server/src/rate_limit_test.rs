use super::*;

#[test]
fn per_client_allows_up_to_limit() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    for i in 0..DEFAULT_PER_CLIENT_LIMIT {
        assert!(rl.check_and_record_at(client, now).is_ok(), "request {i} should succeed");
    }
    assert!(matches!(
        rl.check_and_record_at(client, now),
        Err(RateLimitError::PerClientExceeded { .. })
    ));
}

#[test]
fn global_allows_up_to_limit() {
    let rl = RateLimiter::new();
    let now = Instant::now();

    // Use distinct clients to avoid hitting per-client limit first.
    for i in 0..DEFAULT_GLOBAL_LIMIT {
        let client = Uuid::new_v4();
        assert!(rl.check_and_record_at(client, now).is_ok(), "request {i} should succeed");
    }
    let client = Uuid::new_v4();
    assert!(matches!(
        rl.check_and_record_at(client, now),
        Err(RateLimitError::GlobalExceeded { .. })
    ));
}

#[test]
fn token_budget_exceeded() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    rl.record_tokens_at(client, DEFAULT_TOKEN_BUDGET, 0, now);

    assert!(matches!(
        rl.check_token_budget_at(client, now),
        Err(RateLimitError::TokenBudgetExceeded { .. })
    ));
}

#[test]
fn window_expiry_allows_new_requests() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let start = Instant::now();

    // Fill up per-client limit.
    for _ in 0..DEFAULT_PER_CLIENT_LIMIT {
        rl.check_and_record_at(client, start).unwrap();
    }
    assert!(rl.check_and_record_at(client, start).is_err());

    // After the window passes, requests should succeed again.
    let after_window = start + Duration::from_secs(DEFAULT_PER_CLIENT_WINDOW_SECS) + Duration::from_millis(1);
    assert!(rl.check_and_record_at(client, after_window).is_ok());
}

#[test]
fn distinct_clients_do_not_interfere() {
    let rl = RateLimiter::new();
    let client_a = Uuid::new_v4();
    let client_b = Uuid::new_v4();
    let now = Instant::now();

    // Fill up client A.
    for _ in 0..DEFAULT_PER_CLIENT_LIMIT {
        rl.check_and_record_at(client_a, now).unwrap();
    }
    assert!(rl.check_and_record_at(client_a, now).is_err());

    // Client B should still be able to make requests.
    assert!(rl.check_and_record_at(client_b, now).is_ok());
}

// =============================================================================
// Default trait
// =============================================================================

#[test]
fn default_creates_usable_limiter() {
    let rl = RateLimiter::default();
    let client = Uuid::new_v4();
    assert!(rl.check_and_record(client).is_ok());
}

// =============================================================================
// Exact boundary rejection
// =============================================================================

#[test]
fn per_client_exact_boundary_rejects_at_limit() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    // Fill exactly to limit.
    for _ in 0..DEFAULT_PER_CLIENT_LIMIT {
        rl.check_and_record_at(client, now).unwrap();
    }
    // Request at exactly the limit count should fail.
    let result = rl.check_and_record_at(client, now);
    assert!(result.is_err());
}

#[test]
fn global_exact_boundary_rejects_at_limit() {
    let rl = RateLimiter::new();
    let now = Instant::now();

    for _ in 0..DEFAULT_GLOBAL_LIMIT {
        rl.check_and_record_at(Uuid::new_v4(), now).unwrap();
    }
    let result = rl.check_and_record_at(Uuid::new_v4(), now);
    assert!(result.is_err());
}

#[test]
fn token_budget_exact_boundary_rejects_at_limit() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    rl.record_tokens_at(client, DEFAULT_TOKEN_BUDGET, 0, now);
    let result = rl.check_token_budget_at(client, now);
    assert!(result.is_err());
}

// =============================================================================
// Error Display messages
// =============================================================================

#[test]
fn per_client_error_display() {
    let err = RateLimitError::PerClientExceeded { limit: 10, window_secs: 60 };
    let msg = err.to_string();
    assert!(msg.contains("per-client"));
    assert!(msg.contains("10"));
    assert!(msg.contains("60"));
}

#[test]
fn global_error_display() {
    let err = RateLimitError::GlobalExceeded { limit: 20, window_secs: 60 };
    let msg = err.to_string();
    assert!(msg.contains("global"));
    assert!(msg.contains("20"));
}

#[test]
fn token_budget_error_display() {
    let err = RateLimitError::TokenBudgetExceeded { budget: 50_000, window_secs: 3600 };
    let msg = err.to_string();
    assert!(msg.contains("token budget"));
    assert!(msg.contains("50000"));
}

// =============================================================================
// Zero tokens budget check passes
// =============================================================================

#[test]
fn zero_tokens_recorded_budget_ok() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    rl.record_tokens_at(client, 0, 0, now);
    assert!(rl.check_token_budget_at(client, now).is_ok());
}

// =============================================================================
// N distinct clients exhaust global
// =============================================================================

#[test]
fn many_distinct_clients_exhaust_global() {
    let rl = RateLimiter::new();
    let now = Instant::now();

    // Each distinct client uses 1 request.
    for _ in 0..DEFAULT_GLOBAL_LIMIT {
        rl.check_and_record_at(Uuid::new_v4(), now).unwrap();
    }
    // Next distinct client should be rejected by global limit.
    let result = rl.check_and_record_at(Uuid::new_v4(), now);
    assert!(matches!(result, Err(RateLimitError::GlobalExceeded { .. })));
}

// =============================================================================
// Token window expiry
// =============================================================================

#[test]
fn token_window_expiry_allows_new_usage() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let start = Instant::now();

    rl.record_tokens_at(client, DEFAULT_TOKEN_BUDGET, 0, start);
    assert!(rl.check_token_budget_at(client, start).is_err());

    // After the token window passes, budget should reset.
    let after_window = start + Duration::from_secs(DEFAULT_TOKEN_WINDOW_SECS) + Duration::from_millis(1);
    assert!(rl.check_token_budget_at(client, after_window).is_ok());
}

#[test]
fn token_reservation_blocks_concurrent_budget_oversubscription() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    assert!(rl.reserve_token_budget_at(client, 30_000, now).is_ok());
    assert!(matches!(
        rl.reserve_token_budget_at(client, 30_000, now),
        Err(RateLimitError::TokenBudgetExceeded { .. })
    ));
}

#[test]
fn release_reserved_tokens_restores_capacity() {
    let rl = RateLimiter::new();
    let client = Uuid::new_v4();
    let now = Instant::now();

    assert!(rl.reserve_token_budget_at(client, 30_000, now).is_ok());
    assert!(rl.reserve_token_budget_at(client, 20_000, now).is_ok());
    assert!(rl.reserve_token_budget_at(client, 1, now).is_err());

    rl.release_reserved_tokens_at(client, 20_000, now);
    assert!(rl.reserve_token_budget_at(client, 10_000, now).is_ok());
}

// =============================================================================
// Global window expiry
// =============================================================================

#[test]
fn global_window_expiry_allows_new_requests() {
    let rl = RateLimiter::new();
    let start = Instant::now();

    for _ in 0..DEFAULT_GLOBAL_LIMIT {
        rl.check_and_record_at(Uuid::new_v4(), start).unwrap();
    }
    assert!(rl.check_and_record_at(Uuid::new_v4(), start).is_err());

    let after_window = start + Duration::from_secs(DEFAULT_GLOBAL_WINDOW_SECS) + Duration::from_millis(1);
    assert!(rl.check_and_record_at(Uuid::new_v4(), after_window).is_ok());
}
