-- Tasks table: Core entity for the Kanban system
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    stage TEXT NOT NULL DEFAULT 'backlog',
    project_path TEXT NOT NULL,
    session_id TEXT,
    priority INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Sessions table: Claude CLI session tracking
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT,
    ended_at TEXT,
    last_snapshot_id TEXT,
    error_message TEXT
);

-- Snapshots table: Git-based task snapshots
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    session_id TEXT REFERENCES sessions(id),
    commit_hash TEXT,
    message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Token usage table: Analytics data
CREATE TABLE token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT REFERENCES tasks(id),
    session_id TEXT REFERENCES sessions(id),
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    model TEXT,
    timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Stage history: Track task movements for analytics
CREATE TABLE stage_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    from_stage TEXT,
    to_stage TEXT NOT NULL,
    moved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX idx_tasks_stage ON tasks(stage);
CREATE INDEX idx_tasks_project ON tasks(project_path);
CREATE INDEX idx_sessions_task ON sessions(task_id);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_tokens_task ON token_usage(task_id);
CREATE INDEX idx_tokens_session ON token_usage(session_id);
