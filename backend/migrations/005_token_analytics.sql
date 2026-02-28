-- Note: token_usage table from migration 001 stores aggregate totals per session.
-- token_events stores granular per-JSONL-line events. Both are maintained;
-- token_events is the authoritative source for the analytics API.

-- Token events: one row per JSONL line from Claude stdout
-- Captures granular token usage per tool call and message turn
CREATE TABLE token_events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id    TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    task_id       TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type    TEXT NOT NULL CHECK (event_type IN ('assistant', 'result', 'tool', 'system')),
    tool_name     TEXT,
    file_ext      TEXT,
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    model         TEXT,
    sequence_no   INTEGER NOT NULL DEFAULT 0,
    timestamp     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Session metrics: project stats captured at session start and updated during run
CREATE TABLE session_metrics (
    session_id     TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    project_files  INTEGER NOT NULL DEFAULT 0,
    project_loc    INTEGER NOT NULL DEFAULT 0,
    lines_written  INTEGER NOT NULL DEFAULT 0,
    lines_deleted  INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Single-column indexes for point lookups and simple filters
CREATE INDEX idx_token_events_session   ON token_events(session_id);
CREATE INDEX idx_token_events_task      ON token_events(task_id);
CREATE INDEX idx_token_events_timestamp ON token_events(timestamp);
CREATE INDEX idx_token_events_tool      ON token_events(tool_name);
CREATE INDEX idx_token_events_ext       ON token_events(file_ext);

-- Composite indexes for common analytics GROUP BY patterns
CREATE INDEX idx_token_events_session_tool ON token_events(session_id, tool_name);
CREATE INDEX idx_token_events_task_ts      ON token_events(task_id, timestamp);
