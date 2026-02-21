//! Session and WS-ticket management.
//!
//! ARCHITECTURE
//! ============
//! HTTP auth uses long-lived session tokens, while websocket upgrades use
//! one-time short-lived tickets to avoid sending cookies over WS query params.
//!
//! TRADE-OFFS
//! ==========
//! Ticket consumption is destructive (`DELETE ... RETURNING`) to guarantee
//! single use; this favors replay safety over reconnect convenience.

use std::fmt::Write;

use rand::Rng;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Generate a cryptographically random 32-byte hex token.
#[must_use]
pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes_to_hex(&bytes)
}

/// Generate a short-lived 16-byte hex WS ticket.
#[must_use]
pub(crate) fn generate_ws_ticket() -> String {
    let bytes: [u8; 16] = rand::rng().random();
    bytes_to_hex(&bytes)
}

/// User row returned from session validation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionUser {
    /// Unique user identifier.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Avatar image URL, if available.
    pub avatar_url: Option<String>,
    /// Assigned presence color (hex).
    pub color: String,
    /// Authentication method used to create the session (e.g. `"github"`, `"email"`).
    pub auth_method: String,
}

/// Create a session for the given user, returning the token.
pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    let token = generate_token();
    sqlx::query("INSERT INTO sessions (token, user_id) VALUES ($1, $2)")
        .bind(&token)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(token)
}

/// Validate a session token and return the associated user.
pub async fn validate_session(pool: &PgPool, token: &str) -> Result<Option<SessionUser>, sqlx::Error> {
    let row = sqlx::query(
        r"SELECT
              u.id,
              u.name,
              u.avatar_url,
              u.color,
              CASE
                  WHEN u.github_id IS NOT NULL THEN 'github'
                  WHEN u.email IS NOT NULL THEN 'email'
                  ELSE 'session'
              END AS auth_method
          FROM sessions s
          JOIN users u ON u.id = s.user_id
          WHERE s.token = $1 AND s.expires_at > now()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| SessionUser {
        id: r.get("id"),
        name: r.get("name"),
        avatar_url: r.get("avatar_url"),
        color: r.get("color"),
        auth_method: r.get("auth_method"),
    }))
}

/// Delete a session by token.
pub async fn delete_session(pool: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Create a short-lived WS ticket for the given user.
pub async fn create_ws_ticket(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    let ticket = generate_ws_ticket();
    sqlx::query("INSERT INTO ws_tickets (ticket, user_id) VALUES ($1, $2)")
        .bind(&ticket)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(ticket)
}

/// Consume a WS ticket atomically, returning the `user_id` if valid.
pub async fn consume_ws_ticket(pool: &PgPool, ticket: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let row = sqlx::query("DELETE FROM ws_tickets WHERE ticket = $1 AND expires_at > now() RETURNING user_id")
        .bind(ticket)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get("user_id")))
}

#[cfg(test)]
#[path = "session_test.rs"]
mod tests;
