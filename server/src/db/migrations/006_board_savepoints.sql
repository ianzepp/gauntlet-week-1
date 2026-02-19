CREATE TABLE IF NOT EXISTS board_savepoints (
    id          UUID PRIMARY KEY,
    board_id    UUID NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    seq         BIGINT NOT NULL,
    ts          BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM now()) * 1000)::BIGINT,
    created_by  UUID,
    is_auto     BOOLEAN NOT NULL DEFAULT FALSE,
    reason      TEXT NOT NULL DEFAULT '',
    label       TEXT,
    snapshot    JSONB NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_board_savepoints_board_seq ON board_savepoints(board_id, seq DESC);
