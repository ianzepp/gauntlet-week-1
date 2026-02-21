//! In-memory rate limiting for AI requests.
//!
//! DESIGN
//! ======
//! Sliding-window counters backed by `HashMap<Uuid, VecDeque<Instant>>`.
//! Three limits enforced (per issue #12 spec):
//! - Per-client: 10 AI requests/min
//! - Global: 20 LLM API calls/min
//! - Token budget: 50k tokens/user/hour
//!
//! TRADE-OFFS
//! ==========
//! Token budgeting uses reservations to prevent concurrent requests from
//! oversubscribing quota. This can transiently reserve more than eventual
//! usage, but we release/settle reservations immediately after each call.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use uuid::Uuid;

const DEFAULT_PER_CLIENT_LIMIT: usize = 10;
const DEFAULT_PER_CLIENT_WINDOW_SECS: u64 = 60;

const DEFAULT_GLOBAL_LIMIT: usize = 20;
const DEFAULT_GLOBAL_WINDOW_SECS: u64 = 60;

const DEFAULT_TOKEN_BUDGET: u64 = 50_000;
const DEFAULT_TOKEN_WINDOW_SECS: u64 = 3600;

#[derive(Clone, Copy)]
struct RateLimitConfig {
    per_client_limit: usize,
    per_client_window: Duration,
    global_limit: usize,
    global_window: Duration,
    token_budget: u64,
    token_window: Duration,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        let per_client_window_secs = env_parse("RATE_LIMIT_PER_CLIENT_WINDOW_SECS", DEFAULT_PER_CLIENT_WINDOW_SECS);
        let global_window_secs = env_parse("RATE_LIMIT_GLOBAL_WINDOW_SECS", DEFAULT_GLOBAL_WINDOW_SECS);
        let token_window_secs = env_parse("RATE_LIMIT_TOKEN_WINDOW_SECS", DEFAULT_TOKEN_WINDOW_SECS);

        Self {
            per_client_limit: env_parse("RATE_LIMIT_PER_CLIENT", DEFAULT_PER_CLIENT_LIMIT),
            per_client_window: Duration::from_secs(per_client_window_secs),
            global_limit: env_parse("RATE_LIMIT_GLOBAL", DEFAULT_GLOBAL_LIMIT),
            global_window: Duration::from_secs(global_window_secs),
            token_budget: env_parse("RATE_LIMIT_TOKEN_BUDGET", DEFAULT_TOKEN_BUDGET),
            token_window: Duration::from_secs(token_window_secs),
        }
    }
}

fn env_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

// =============================================================================
// ERROR TYPE
// =============================================================================

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum RateLimitError {
    #[error("per-client rate limit exceeded (max {limit} requests/{window_secs}s)")]
    PerClientExceeded { limit: usize, window_secs: u64 },
    #[error("global rate limit exceeded (max {limit} requests/{window_secs}s)")]
    GlobalExceeded { limit: usize, window_secs: u64 },
    #[error("token budget exceeded (max {budget} tokens/{window_secs}s)")]
    TokenBudgetExceeded { budget: u64, window_secs: u64 },
}

// =============================================================================
// RATE LIMITER
// =============================================================================

#[derive(Clone)]
pub struct RateLimiter {
    inner: std::sync::Arc<Mutex<RateLimiterInner>>,
    config: RateLimitConfig,
}

struct RateLimiterInner {
    /// Per-client request timestamps.
    client_requests: HashMap<Uuid, VecDeque<Instant>>,
    /// Global request timestamps.
    global_requests: VecDeque<Instant>,
    /// Per-client token usage: (timestamp, `token_count`).
    client_tokens: HashMap<Uuid, VecDeque<(Instant, u64)>>,
    /// Per-client in-flight token reservations: (timestamp, reserved tokens).
    client_token_reservations: HashMap<Uuid, VecDeque<(Instant, u64)>>,
}

impl RateLimiter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(RateLimiterInner {
                client_requests: HashMap::new(),
                global_requests: VecDeque::new(),
                client_tokens: HashMap::new(),
                client_token_reservations: HashMap::new(),
            })),
            config: RateLimitConfig::from_env(),
        }
    }

    /// Check both per-client and global rate limits, then record the request.
    pub fn check_and_record(&self, client_id: Uuid) -> Result<(), RateLimitError> {
        self.check_and_record_at(client_id, Instant::now())
    }

    /// Internal: check + record with explicit timestamp (for testing).
    fn check_and_record_at(&self, client_id: Uuid, now: Instant) -> Result<(), RateLimitError> {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cfg = self.config;

        // Prune and check global first (no borrow conflict).
        prune_window(&mut inner.global_requests, now, cfg.global_window);
        if inner.global_requests.len() >= cfg.global_limit {
            return Err(RateLimitError::GlobalExceeded {
                limit: cfg.global_limit,
                window_secs: cfg.global_window.as_secs(),
            });
        }

        // Prune and check per-client.
        let client_deque = inner.client_requests.entry(client_id).or_default();
        prune_window(client_deque, now, cfg.per_client_window);
        if client_deque.len() >= cfg.per_client_limit {
            return Err(RateLimitError::PerClientExceeded {
                limit: cfg.per_client_limit,
                window_secs: cfg.per_client_window.as_secs(),
            });
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
        self.reserve_token_budget_at(client_id, 0, now)
    }

    /// Reserve token budget before issuing an LLM call.
    ///
    /// This reservation is atomic with the budget check so concurrent requests
    /// see each other's in-flight usage.
    pub fn reserve_token_budget(&self, client_id: Uuid, reserved_tokens: u64) -> Result<(), RateLimitError> {
        self.reserve_token_budget_at(client_id, reserved_tokens, Instant::now())
    }

    fn reserve_token_budget_at(
        &self,
        client_id: Uuid,
        reserved_tokens: u64,
        now: Instant,
    ) -> Result<(), RateLimitError> {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cfg = self.config;
        let used_tokens: u64 = {
            let token_deque = inner.client_tokens.entry(client_id).or_default();
            prune_token_window(token_deque, now, cfg.token_window);
            token_deque.iter().map(|(_, t)| t).sum()
        };
        let reserved_total: u64 = {
            let reservation_deque = inner
                .client_token_reservations
                .entry(client_id)
                .or_default();
            prune_token_window(reservation_deque, now, cfg.token_window);
            reservation_deque.iter().map(|(_, t)| t).sum()
        };
        let Some(projected_total) = used_tokens
            .checked_add(reserved_total)
            .and_then(|n| n.checked_add(reserved_tokens))
        else {
            return Err(RateLimitError::TokenBudgetExceeded {
                budget: cfg.token_budget,
                window_secs: cfg.token_window.as_secs(),
            });
        };
        let exceeds_budget = if reserved_tokens == 0 {
            projected_total >= cfg.token_budget
        } else {
            projected_total > cfg.token_budget
        };
        if exceeds_budget {
            return Err(RateLimitError::TokenBudgetExceeded {
                budget: cfg.token_budget,
                window_secs: cfg.token_window.as_secs(),
            });
        }
        if reserved_tokens > 0 {
            inner
                .client_token_reservations
                .entry(client_id)
                .or_default()
                .push_back((now, reserved_tokens));
        }
        Ok(())
    }

    /// Record token usage after an LLM response.
    pub fn record_tokens(&self, client_id: Uuid, tokens: u64, reserved_tokens: u64) {
        self.record_tokens_at(client_id, tokens, reserved_tokens, Instant::now());
    }

    fn record_tokens_at(&self, client_id: Uuid, tokens: u64, reserved_tokens: u64, now: Instant) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cfg = self.config;
        {
            let reservation_deque = inner
                .client_token_reservations
                .entry(client_id)
                .or_default();
            prune_token_window(reservation_deque, now, cfg.token_window);
            consume_reserved_tokens(reservation_deque, reserved_tokens);
        }
        {
            let token_deque = inner.client_tokens.entry(client_id).or_default();
            prune_token_window(token_deque, now, cfg.token_window);
            token_deque.push_back((now, tokens));
        }
    }

    /// Release reservation for failed or canceled LLM calls.
    pub fn release_reserved_tokens(&self, client_id: Uuid, reserved_tokens: u64) {
        self.release_reserved_tokens_at(client_id, reserved_tokens, Instant::now());
    }

    fn release_reserved_tokens_at(&self, client_id: Uuid, reserved_tokens: u64, now: Instant) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cfg = self.config;
        let reservation_deque = inner
            .client_token_reservations
            .entry(client_id)
            .or_default();
        prune_token_window(reservation_deque, now, cfg.token_window);
        consume_reserved_tokens(reservation_deque, reserved_tokens);
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

fn consume_reserved_tokens(deque: &mut VecDeque<(Instant, u64)>, mut amount: u64) {
    while amount > 0 {
        let Some((_, front_tokens)) = deque.front_mut() else {
            break;
        };
        if *front_tokens <= amount {
            amount -= *front_tokens;
            deque.pop_front();
        } else {
            *front_tokens -= amount;
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
