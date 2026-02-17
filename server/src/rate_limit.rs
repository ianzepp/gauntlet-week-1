//! In-memory rate limiting for AI requests.
//!
//! DESIGN
//! ======
//! Sliding-window counters backed by `HashMap<Uuid, VecDeque<Instant>>`.
//! Three limits enforced (per issue #12 spec):
//! - Per-client: 10 AI requests/min
//! - Global: 20 LLM API calls/min
//! - Token budget: 50k tokens/user/hour

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use uuid::Uuid;

const PER_CLIENT_LIMIT: usize = 10;
const PER_CLIENT_WINDOW: Duration = Duration::from_secs(60);

const GLOBAL_LIMIT: usize = 20;
const GLOBAL_WINDOW: Duration = Duration::from_secs(60);

const TOKEN_BUDGET: u64 = 50_000;
const TOKEN_WINDOW: Duration = Duration::from_secs(3600);

// =============================================================================
// ERROR TYPE
// =============================================================================

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum RateLimitError {
    #[error("per-client rate limit exceeded (max {PER_CLIENT_LIMIT} requests/min)")]
    PerClientExceeded,
    #[error("global rate limit exceeded (max {GLOBAL_LIMIT} requests/min)")]
    GlobalExceeded,
    #[error("token budget exceeded (max {TOKEN_BUDGET} tokens/hour)")]
    TokenBudgetExceeded,
}

// =============================================================================
// RATE LIMITER
// =============================================================================

#[derive(Clone)]
pub struct RateLimiter {
    inner: std::sync::Arc<Mutex<RateLimiterInner>>,
}

struct RateLimiterInner {
    /// Per-client request timestamps.
    client_requests: HashMap<Uuid, VecDeque<Instant>>,
    /// Global request timestamps.
    global_requests: VecDeque<Instant>,
    /// Per-client token usage: (timestamp, `token_count`).
    client_tokens: HashMap<Uuid, VecDeque<(Instant, u64)>>,
}

impl RateLimiter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(RateLimiterInner {
                client_requests: HashMap::new(),
                global_requests: VecDeque::new(),
                client_tokens: HashMap::new(),
            })),
        }
    }

    /// Check both per-client and global rate limits, then record the request.
    pub fn check_and_record(&self, client_id: Uuid) -> Result<(), RateLimitError> {
        self.check_and_record_at(client_id, Instant::now())
    }

    /// Internal: check + record with explicit timestamp (for testing).
    fn check_and_record_at(&self, client_id: Uuid, now: Instant) -> Result<(), RateLimitError> {
        let mut inner = self.inner.lock().unwrap();

        // Prune and check global first (no borrow conflict).
        prune_window(&mut inner.global_requests, now, GLOBAL_WINDOW);
        if inner.global_requests.len() >= GLOBAL_LIMIT {
            return Err(RateLimitError::GlobalExceeded);
        }

        // Prune and check per-client.
        let client_deque = inner.client_requests.entry(client_id).or_default();
        prune_window(client_deque, now, PER_CLIENT_WINDOW);
        if client_deque.len() >= PER_CLIENT_LIMIT {
            return Err(RateLimitError::PerClientExceeded);
        }

        // Record.
        client_deque.push_back(now);
        inner.global_requests.push_back(now);

        Ok(())
    }

    /// Check if the client's token budget allows another request.
    pub fn check_token_budget(&self, client_id: Uuid) -> Result<(), RateLimitError> {
        self.check_token_budget_at(client_id, Instant::now())
    }

    fn check_token_budget_at(&self, client_id: Uuid, now: Instant) -> Result<(), RateLimitError> {
        let mut inner = self.inner.lock().unwrap();
        let token_deque = inner.client_tokens.entry(client_id).or_default();
        prune_token_window(token_deque, now, TOKEN_WINDOW);

        let total: u64 = token_deque.iter().map(|(_, t)| t).sum();
        if total >= TOKEN_BUDGET {
            return Err(RateLimitError::TokenBudgetExceeded);
        }
        Ok(())
    }

    /// Record token usage after an LLM response.
    pub fn record_tokens(&self, client_id: Uuid, tokens: u64) {
        self.record_tokens_at(client_id, tokens, Instant::now());
    }

    fn record_tokens_at(&self, client_id: Uuid, tokens: u64, now: Instant) {
        let mut inner = self.inner.lock().unwrap();
        let token_deque = inner.client_tokens.entry(client_id).or_default();
        token_deque.push_back((now, tokens));
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HELPERS
// =============================================================================

fn prune_window(deque: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while let Some(&front) = deque.front() {
        if now.duration_since(front) > window {
            deque.pop_front();
        } else {
            break;
        }
    }
}

fn prune_token_window(deque: &mut VecDeque<(Instant, u64)>, now: Instant, window: Duration) {
    while let Some(&(front, _)) = deque.front() {
        if now.duration_since(front) > window {
            deque.pop_front();
        } else {
            break;
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
#[path = "rate_limit_test.rs"]
mod tests;
