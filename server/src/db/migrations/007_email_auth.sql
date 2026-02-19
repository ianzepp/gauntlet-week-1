ALTER TABLE users
ADD COLUMN IF NOT EXISTS email TEXT UNIQUE;

CREATE TABLE IF NOT EXISTS email_login_codes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT NOT NULL,
    code_hash   TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT (now() + INTERVAL '10 minutes'),
    consumed_at TIMESTAMPTZ,
    attempts    INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_email_login_codes_email_active
    ON email_login_codes(email, created_at DESC)
    WHERE consumed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_email_login_codes_expires
    ON email_login_codes(expires_at);
