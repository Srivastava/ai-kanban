-- backend/migrations/015_task_attachments.sql
CREATE TABLE IF NOT EXISTS task_attachments (
    id           TEXT PRIMARY KEY,
    task_id      TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    mime_type    TEXT NOT NULL DEFAULT 'application/octet-stream',
    size_bytes   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_task_attachments_task_id ON task_attachments(task_id);
