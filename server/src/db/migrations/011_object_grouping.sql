ALTER TABLE board_objects
    ADD COLUMN IF NOT EXISTS group_id UUID;

CREATE INDEX IF NOT EXISTS idx_board_objects_group_id
    ON board_objects (group_id);
