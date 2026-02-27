-- Logs table: Unified logging for backend and frontend
CREATE TABLE logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,           -- DEBUG, INFO, WARN, ERROR
    message TEXT NOT NULL,
    target TEXT,                   -- module path (e.g., "api::tasks", "frontend:components/Kanban")
    source TEXT NOT NULL DEFAULT 'backend',  -- 'backend' or 'frontend'
    task_id TEXT,                  -- optional context
    session_id TEXT,               -- optional context
    metadata TEXT,                 -- JSON blob for extra fields
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX idx_logs_timestamp ON logs(timestamp);
CREATE INDEX idx_logs_level ON logs(level);
CREATE INDEX idx_logs_source ON logs(source);
CREATE INDEX idx_logs_task ON logs(task_id);
CREATE INDEX idx_logs_session ON logs(session_id);
