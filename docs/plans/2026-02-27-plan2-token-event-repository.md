# Token Event Repository Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Rust models and a repository for `token_events` and `session_metrics` tables.

**Architecture:** Follow the existing Repository pattern: `TokenEventRepository` and `SessionMetricsRepository` structs wrapping `SqlitePool`, with typed model structs using `sqlx::FromRow`. Register in `backend/src/db/mod.rs` and `backend/src/models/mod.rs`.

**Tech Stack:** Rust, SQLx 0.7, SQLite, chrono, serde

---

## Context

Existing pattern to follow — look at `backend/src/db/logs.rs` and `backend/src/models/log.rs` as the template.

Files you'll create:
- `backend/src/models/token_event.rs` — TokenEvent, CreateTokenEvent, SessionMetrics structs
- `backend/src/db/token_events.rs` — TokenEventRepository
- `backend/src/db/session_metrics.rs` — SessionMetricsRepository

Files you'll modify:
- `backend/src/models/mod.rs` — add pub use for new types
- `backend/src/db/mod.rs` — add pub use for new repositories

---

## Task 1: Token Event Model

**Files:**
- Create: `backend/src/models/token_event.rs`

**Step 1: Write failing test first**

In `backend/src/models/token_event.rs`, before writing the struct, we verify the model compiles with our expected fields. We do this by writing the file and checking `cargo build`.

**Step 2: Create the model file**

Create `backend/src/models/token_event.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single token event from Claude's JSONL stdout stream
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TokenEvent {
    pub id: i64,
    pub session_id: String,
    pub task_id: String,
    pub event_type: String,       // 'assistant', 'result', 'tool', 'system'
    pub tool_name: Option<String>,
    pub file_ext: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: Option<String>,
    pub sequence_no: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenEvent {
    pub session_id: String,
    pub task_id: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub file_ext: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: Option<String>,
    pub sequence_no: Option<i64>,
}

/// Per-session project metrics (captured at start, updated during run)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionMetrics {
    pub session_id: String,
    pub project_files: i64,
    pub project_loc: i64,
    pub lines_written: i64,
    pub lines_deleted: i64,
    pub updated_at: DateTime<Utc>,
}
```

**Step 3: Register model in mod.rs**

Open `backend/src/models/mod.rs`. Add at the bottom:

```rust
mod token_event;
pub use token_event::{CreateTokenEvent, SessionMetrics, TokenEvent};
```

**Step 4: Verify compilation**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | grep -E "^error"
```

Expected: no output (no errors).

---

## Task 2: Token Event Repository

**Files:**
- Create: `backend/src/db/token_events.rs`

**Step 1: Write test file first**

Create `backend/tests/token_events_test.rs`:

```rust
use ai_kanban_backend::db::{create_pool, TokenEventRepository};
use ai_kanban_backend::models::CreateTokenEvent;

async fn setup_db() -> sqlx::SqlitePool {
    let pool = create_pool(":memory:").await.expect("Failed to create test pool");
    pool
}

#[tokio::test]
async fn test_create_token_event() {
    let pool = setup_db().await;
    let repo = TokenEventRepository::new(pool);

    // We need a session and task first (FK constraints)
    // For this test, disable FK checks or insert parent rows
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(repo.pool())
        .await
        .unwrap();

    let event = repo
        .create(CreateTokenEvent {
            session_id: "sess-1".to_string(),
            task_id: "task-1".to_string(),
            event_type: "assistant".to_string(),
            tool_name: Some("Read".to_string()),
            file_ext: Some(".rs".to_string()),
            input_tokens: 100,
            output_tokens: 50,
            model: Some("claude-sonnet-4-6".to_string()),
            sequence_no: Some(0),
        })
        .await
        .expect("Failed to create token event");

    assert_eq!(event.session_id, "sess-1");
    assert_eq!(event.input_tokens, 100);
    assert_eq!(event.tool_name, Some("Read".to_string()));
}

#[tokio::test]
async fn test_list_by_session() {
    let pool = setup_db().await;
    let repo = TokenEventRepository::new(pool);

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(repo.pool())
        .await
        .unwrap();

    // Insert two events for same session, one for different
    repo.create(CreateTokenEvent {
        session_id: "sess-a".to_string(),
        task_id: "task-1".to_string(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 100,
        output_tokens: 20,
        model: None,
        sequence_no: Some(0),
    })
    .await
    .unwrap();

    repo.create(CreateTokenEvent {
        session_id: "sess-a".to_string(),
        task_id: "task-1".to_string(),
        event_type: "result".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 200,
        output_tokens: 80,
        model: None,
        sequence_no: Some(1),
    })
    .await
    .unwrap();

    repo.create(CreateTokenEvent {
        session_id: "sess-b".to_string(),
        task_id: "task-2".to_string(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 50,
        output_tokens: 10,
        model: None,
        sequence_no: Some(0),
    })
    .await
    .unwrap();

    let events = repo.list_by_session("sess-a").await.unwrap();
    assert_eq!(events.len(), 2);
    // Ordered by sequence_no ASC
    assert_eq!(events[0].sequence_no, Some(0));
    assert_eq!(events[1].sequence_no, Some(1));

    let other = repo.list_by_session("sess-b").await.unwrap();
    assert_eq!(other.len(), 1);
}
```

**Step 2: Run test to verify it fails**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test token_events 2>&1 | tail -5
```

Expected: compile error — `TokenEventRepository` doesn't exist yet.

**Step 3: Implement the repository**

Create `backend/src/db/token_events.rs`:

```rust
use crate::models::{CreateTokenEvent, TokenEvent};
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct TokenEventRepository {
    pool: SqlitePool,
}

impl TokenEventRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Expose pool for tests (FK disabling)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create(&self, create: CreateTokenEvent) -> Result<TokenEvent> {
        let now = chrono::Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO token_events
                (session_id, task_id, event_type, tool_name, file_ext,
                 input_tokens, output_tokens, model, sequence_no, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&create.session_id)
        .bind(&create.task_id)
        .bind(&create.event_type)
        .bind(&create.tool_name)
        .bind(&create.file_ext)
        .bind(create.input_tokens)
        .bind(create.output_tokens)
        .bind(&create.model)
        .bind(create.sequence_no)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(TokenEvent {
            id: result.last_insert_rowid(),
            session_id: create.session_id,
            task_id: create.task_id,
            event_type: create.event_type,
            tool_name: create.tool_name,
            file_ext: create.file_ext,
            input_tokens: create.input_tokens,
            output_tokens: create.output_tokens,
            model: create.model,
            sequence_no: create.sequence_no,
            timestamp: now,
        })
    }

    /// All events for a session, ordered by sequence_no for timeline charts
    pub async fn list_by_session(&self, session_id: &str) -> Result<Vec<TokenEvent>> {
        let events = sqlx::query_as::<_, TokenEvent>(
            "SELECT * FROM token_events WHERE session_id = ? ORDER BY sequence_no ASC, id ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(events)
    }

    /// All events for a task (across all sessions)
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<TokenEvent>> {
        let events = sqlx::query_as::<_, TokenEvent>(
            "SELECT * FROM token_events WHERE task_id = ? ORDER BY timestamp ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(events)
    }

    /// Batch insert — used by the JSONL parser to avoid per-line round trips
    pub async fn create_batch(&self, events: Vec<CreateTokenEvent>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        for event in events {
            self.create(event).await?;
        }
        Ok(())
    }
}
```

**Step 4: Register in db/mod.rs**

Open `backend/src/db/mod.rs`. Add:

```rust
mod token_events;
pub use token_events::TokenEventRepository;
```

**Step 5: Run tests to verify they pass**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test token_events 2>&1
```

Expected: `test test_create_token_event ... ok` and `test test_list_by_session ... ok`

---

## Task 3: Session Metrics Repository

**Files:**
- Create: `backend/src/db/session_metrics.rs`

**Step 1: Write test**

Add to `backend/tests/token_events_test.rs`:

```rust
use ai_kanban_backend::db::SessionMetricsRepository;

#[tokio::test]
async fn test_upsert_session_metrics() {
    let pool = setup_db().await;
    let repo = SessionMetricsRepository::new(pool);

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(repo.pool())
        .await
        .unwrap();

    // Insert initial metrics
    repo.upsert("sess-1", 42, 1500).await.unwrap();

    let m = repo.find("sess-1").await.unwrap().unwrap();
    assert_eq!(m.project_files, 42);
    assert_eq!(m.project_loc, 1500);
    assert_eq!(m.lines_written, 0);

    // Increment lines_written
    repo.add_lines_written("sess-1", 10).await.unwrap();
    let m2 = repo.find("sess-1").await.unwrap().unwrap();
    assert_eq!(m2.lines_written, 10);

    // Increment again
    repo.add_lines_written("sess-1", 5).await.unwrap();
    let m3 = repo.find("sess-1").await.unwrap().unwrap();
    assert_eq!(m3.lines_written, 15);
}
```

**Step 2: Run test to verify it fails**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test upsert_session_metrics 2>&1 | tail -5
```

Expected: compile error.

**Step 3: Implement SessionMetricsRepository**

Create `backend/src/db/session_metrics.rs`:

```rust
use crate::models::SessionMetrics;
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SessionMetricsRepository {
    pool: SqlitePool,
}

impl SessionMetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Insert or replace metrics row for a session
    pub async fn upsert(&self, session_id: &str, project_files: i64, project_loc: i64) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            INSERT INTO session_metrics (session_id, project_files, project_loc, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(session_id) DO UPDATE SET
                project_files = excluded.project_files,
                project_loc   = excluded.project_loc,
                updated_at    = excluded.updated_at
            "#,
        )
        .bind(session_id)
        .bind(project_files)
        .bind(project_loc)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find(&self, session_id: &str) -> Result<Option<SessionMetrics>> {
        let metrics = sqlx::query_as::<_, SessionMetrics>(
            "SELECT * FROM session_metrics WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(metrics)
    }

    pub async fn add_lines_written(&self, session_id: &str, lines: i64) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE session_metrics SET lines_written = lines_written + ?, updated_at = ? WHERE session_id = ?",
        )
        .bind(lines)
        .bind(now.to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_lines_deleted(&self, session_id: &str, lines: i64) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE session_metrics SET lines_deleted = lines_deleted + ?, updated_at = ? WHERE session_id = ?",
        )
        .bind(lines)
        .bind(now.to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

**Step 4: Register in db/mod.rs**

Add to `backend/src/db/mod.rs`:

```rust
mod session_metrics;
pub use session_metrics::SessionMetricsRepository;
```

**Step 5: Run all new tests**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test token_events 2>&1
```

Expected: 3 tests pass.

**Step 6: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add backend/src/models/token_event.rs \
        backend/src/models/mod.rs \
        backend/src/db/token_events.rs \
        backend/src/db/session_metrics.rs \
        backend/src/db/mod.rs \
        backend/tests/token_events_test.rs
git commit -m "feat(db): add TokenEventRepository and SessionMetricsRepository

- TokenEvent model and repository with create/list_by_session/list_by_task
- SessionMetrics model and repository with upsert/add_lines_written/deleted
- Tests for both repositories using in-memory SQLite"
```
