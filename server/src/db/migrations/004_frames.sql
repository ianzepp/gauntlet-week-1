CREATE TABLE IF NOT EXISTS frames (
    seq         BIGSERIAL PRIMARY KEY,
    ts          BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM now()) * 1000)::BIGINT,
    id          UUID NOT NULL,
    parent_id   UUID,
    syscall     TEXT NOT NULL,
    status      TEXT NOT NULL,
    board_id    UUID,
    "from"      TEXT,
    data        JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_frames_board_seq ON frames(board_id, seq);
