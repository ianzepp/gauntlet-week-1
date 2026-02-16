CREATE TABLE IF NOT EXISTS users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id   BIGINT UNIQUE,
    name        TEXT NOT NULL,
    avatar_url  TEXT,
    color       TEXT NOT NULL DEFAULT '#4CAF50',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
