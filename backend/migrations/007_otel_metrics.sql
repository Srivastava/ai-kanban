CREATE TABLE IF NOT EXISTS otel_metrics (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    metric_name       TEXT NOT NULL,
    value             REAL NOT NULL,
    unit              TEXT,
    -- ACTO correlation (null = not from an ACTO-managed session)
    session_id        TEXT REFERENCES sessions(id),
    task_id           TEXT REFERENCES tasks(id),
    -- Original OTel identifiers
    claude_session_id TEXT NOT NULL,
    -- Additional OTel attributes stored as JSON object
    attributes        TEXT NOT NULL DEFAULT '{}',
    -- OTel timestamp (Unix nanoseconds)
    otel_timestamp    INTEGER NOT NULL,
    created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_task_id       ON otel_metrics(task_id);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_session_id    ON otel_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_metric_name   ON otel_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_claude_sid    ON otel_metrics(claude_session_id);
