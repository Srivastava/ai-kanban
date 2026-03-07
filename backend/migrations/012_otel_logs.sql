CREATE TABLE IF NOT EXISTS otel_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_name TEXT NOT NULL DEFAULT '',
    body TEXT,
    severity_text TEXT,
    severity_number INTEGER,
    session_id TEXT,
    task_id TEXT,
    claude_session_id TEXT NOT NULL DEFAULT '',
    attributes TEXT NOT NULL DEFAULT '{}',
    otel_timestamp INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_otel_logs_session_id ON otel_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_otel_logs_task_id ON otel_logs(task_id);
CREATE INDEX IF NOT EXISTS idx_otel_logs_claude_session_id ON otel_logs(claude_session_id);
