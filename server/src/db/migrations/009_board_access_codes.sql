CREATE TABLE IF NOT EXISTS board_access_codes (
    code        TEXT PRIMARY KEY,
    board_id    UUID NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    created_by  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT now() + interval '24 hours'
);

CREATE INDEX IF NOT EXISTS idx_board_access_codes_board ON board_access_codes(board_id);
