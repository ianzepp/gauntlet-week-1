//! Email access-code auth service.
//!
//! Creates and verifies short-lived six-character codes linked to an email.

use rand::Rng;
use resend_rs::Resend;
use resend_rs::types::CreateEmailBaseOptions;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const CODE_LEN: usize = 6;
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const MAX_FAILED_ATTEMPTS: i32 = 5;
const EMAIL_AUTH_TEMPLATE: &str = include_str!("../../templates/email_auth.html");

#[derive(Debug, thiserror::Error)]
pub enum EmailAuthError {
    #[error("invalid email")]
    InvalidEmail,
    #[error("invalid code")]
    InvalidCode,
    #[error("expired or incorrect code")]
    VerificationFailed,
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("email delivery failed: {0}")]
    EmailDelivery(String),
}

#[must_use]
pub fn normalize_email(email: &str) -> Option<String> {
    let normalized = email.trim().to_ascii_lowercase();
    if normalized.is_empty() || !normalized.contains('@') {
        return None;
    }
    let parts = normalized.split('@').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some(normalized)
}

#[must_use]
pub fn normalize_code(code: &str) -> Option<String> {
    let normalized = code.trim().to_ascii_uppercase();
    if normalized.len() != CODE_LEN
        || !normalized
            .chars()
            .all(|c| CODE_ALPHABET.contains(&(c as u8)))
    {
        return None;
    }
    Some(normalized)
}

#[must_use]
pub fn generate_access_code() -> String {
    let mut rng = rand::rng();
    (0..CODE_LEN)
        .map(|_| {
            let idx = rng.random_range(0..CODE_ALPHABET.len());
            CODE_ALPHABET[idx] as char
        })
        .collect()
}

#[must_use]
pub fn hash_access_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    let bytes = hasher.finalize();
    bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

fn name_from_email(email: &str) -> String {
    let local = email
        .split('@')
        .next()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("user");
    local.to_owned()
}

pub async fn request_access_code(pool: &PgPool, email: &str) -> Result<String, EmailAuthError> {
    let normalized = normalize_email(email).ok_or(EmailAuthError::InvalidEmail)?;
    let name = name_from_email(&normalized);

    sqlx::query(
        r"INSERT INTO users (email, name)
          VALUES ($1, $2)
          ON CONFLICT (email) DO UPDATE SET name = users.name",
    )
    .bind(&normalized)
    .bind(name)
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM email_login_codes WHERE email = $1 AND consumed_at IS NULL")
        .bind(&normalized)
        .execute(pool)
        .await?;

    let code = generate_access_code();
    let code_hash = hash_access_code(&code);

    sqlx::query("INSERT INTO email_login_codes (email, code_hash) VALUES ($1, $2)")
        .bind(&normalized)
        .bind(code_hash)
        .execute(pool)
        .await?;

    Ok(code)
}

pub async fn verify_access_code(pool: &PgPool, email: &str, code: &str) -> Result<Uuid, EmailAuthError> {
    let normalized_email = normalize_email(email).ok_or(EmailAuthError::InvalidEmail)?;
    let normalized_code = normalize_code(code).ok_or(EmailAuthError::InvalidCode)?;
    let code_hash = hash_access_code(&normalized_code);

    let update = sqlx::query(
        r"UPDATE email_login_codes
          SET consumed_at = now()
          WHERE id = (
              SELECT id
              FROM email_login_codes
              WHERE email = $1
                AND consumed_at IS NULL
                AND expires_at > now()
              ORDER BY created_at DESC
              LIMIT 1
          )
          AND code_hash = $2
          RETURNING id",
    )
    .bind(&normalized_email)
    .bind(&code_hash)
    .fetch_optional(pool)
    .await?;

    if update.is_none() {
        sqlx::query(
            r"UPDATE email_login_codes
              SET attempts = attempts + 1,
                  consumed_at = CASE WHEN attempts + 1 >= $2 THEN now() ELSE consumed_at END
              WHERE id = (
                  SELECT id
                  FROM email_login_codes
                  WHERE email = $1
                    AND consumed_at IS NULL
                    AND expires_at > now()
                  ORDER BY created_at DESC
                  LIMIT 1
              )",
        )
        .bind(&normalized_email)
        .bind(MAX_FAILED_ATTEMPTS)
        .execute(pool)
        .await?;
        return Err(EmailAuthError::VerificationFailed);
    }

    let user_row = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind(&normalized_email)
        .fetch_optional(pool)
        .await?;

    let Some(user_row) = user_row else {
        return Err(EmailAuthError::VerificationFailed);
    };

    Ok(user_row.get("id"))
}

pub async fn send_access_code_email(
    resend_api_key: &str,
    resend_from: &str,
    to_email: &str,
    code: &str,
) -> Result<(), EmailAuthError> {
    let resend = Resend::new(resend_api_key);
    let to = [to_email];
    let subject = "Your Gauntlet AI Access Code";
    let html = render_email_auth_template(to_email, code);

    let email = CreateEmailBaseOptions::new(resend_from, to, subject).with_html(&html);
    resend
        .emails
        .send(email)
        .await
        .map_err(|e| EmailAuthError::EmailDelivery(e.to_string()))?;
    Ok(())
}

#[must_use]
pub fn render_email_auth_template(email: &str, code: &str) -> String {
    EMAIL_AUTH_TEMPLATE
        .replace("{{EMAIL}}", email)
        .replace("{{CODE}}", code)
}

#[cfg(test)]
#[path = "email_auth_test.rs"]
mod tests;
