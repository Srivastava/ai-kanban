# Token Analytics Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `token_events` and `session_metrics` tables to SQLite via SQLx migration.

**Architecture:** Add a single numbered migration SQL file. SQLx auto-runs all pending migrations at startup via `sqlx::migrate!("./migrations")` in `backend/src/db/pool.rs`. No code changes needed beyond the SQL file itself.

**Tech Stack:** SQLite, SQLx migrations (files in `backend/migrations/`)

---

## Context

Existing migrations:
- `001_initial.sql` — tasks, sessions, snapshots, token_usage, stage_history
- `002_logs.sql` — logs table
- `003_add_context.sql` — adds context column to tasks
- `004_add_comments.sql` — comments table

New file: `005_token_analytics.sql`

The `token_events` table stores one row per JSONL event from Claude stdout (per tool call, per message turn). The `session_metrics` table stores per-session project stats (file count, LOC, lines written).

---

## Task 1: Write the Migration SQL

**Files:**
- Create: `backend/migrations/005_token_analytics.sql`

**Step 1: Create the migration file**

Create `backend/migrations/005_token_analytics.sql` with this exact content:

```sql
-- Token events: one row per JSONL line from Claude stdout
-- Captures granular token usage per tool call and message turn
CREATE TABLE token_events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id    TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    task_id       TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type    TEXT NOT NULL,        -- 'assistant', 'result', 'tool', 'system'
    tool_name     TEXT,                 -- 'Read', 'Write', 'Edit', 'Bash', 'Grep', etc.
    file_ext      TEXT,                 -- '.rs', '.ts', '.py' extracted from tool path args
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    model         TEXT,                 -- e.g. 'claude-sonnet-4-6'
    sequence_no   INTEGER,              -- position in JSONL stream (0-indexed)
    timestamp     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Session metrics: project stats captured at session start and updated during run
CREATE TABLE session_metrics (
    session_id     TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    project_files  INTEGER NOT NULL DEFAULT 0,  -- file count at session start
    project_loc    INTEGER NOT NULL DEFAULT 0,  -- lines of code at session start
    lines_written  INTEGER NOT NULL DEFAULT 0,  -- accumulated from Write/Edit calls
    lines_deleted  INTEGER NOT NULL DEFAULT 0,
    updated_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for analytics query performance
CREATE INDEX idx_token_events_session   ON token_events(session_id);
CREATE INDEX idx_token_events_task      ON token_events(task_id);
CREATE INDEX idx_token_events_timestamp ON token_events(timestamp);
CREATE INDEX idx_token_events_tool      ON token_events(tool_name);
CREATE INDEX idx_token_events_ext       ON token_events(file_ext);
```

**Step 2: Verify the migration runs**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1
```

Expected: compiles without errors. SQLx validates migrations at compile time when `DATABASE_URL` is set.

If you see "error: no DATABASE_URL", set it first:
```bash
export DATABASE_URL="sqlite:data/ai-kanban.db"
cargo build 2>&1
```

**Step 3: Run the app to verify migration applies**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo run &
sleep 3
# Check the tables exist
sqlite3 data/ai-kanban.db ".tables"
```

Expected output includes: `token_events  session_metrics`

Kill the server: `kill %1`

**Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add backend/migrations/005_token_analytics.sql
git commit -m "feat(db): add token_events and session_metrics migration

- token_events: one row per JSONL line from Claude stdout
- session_metrics: project stats per session (files, LOC, lines written/deleted)
- indexes for analytics query performance"
```
