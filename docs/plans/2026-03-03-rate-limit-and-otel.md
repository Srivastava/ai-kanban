# Rate Limit Auto-Resume + OTel Telemetry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** (1) Auto-detect Claude usage limits from stderr, mark the session as stopped, and re-queue it for automatic resume at the reset time. (2) Receive OTLP/HTTP metrics from Claude Code on port 4318, store them in SQLite correlated with ACTO sessions, and surface Dev Activity data in the Analytics page.

**Architecture:**
- Rate limit: stderr scanner sets a shared flag in the manager's stdout loop. Completion handler reads that flag to avoid marking the session `failed` — instead it emits `ClaudeEvent::RateLimited`. A listener in `main.rs` forwards to a new `SessionQueue::schedule_rate_limit_retry()` that sleeps until reset then re-enqueues with `--resume`.
- OTel: second Axum server bound on port 4318 in the same binary. Receives OTLP JSON at `POST /v1/metrics`. Parser extracts metric name, value, and `session.id` attribute; looks up `sessions.claude_session_id`; stores in new `otel_metrics` table. Analytics API and frontend show only ACTO-correlated rows (`task_id IS NOT NULL`).

**Tech Stack:** Rust/Axum/SQLite/tokio, Next.js/React Query, OTLP HTTP+JSON (no protobuf)

---

## Task 1: Rate limit detection — add `extract_rate_limit_reset_at()` to `jsonl_parser.rs`

**Files:**
- Modify: `backend/src/claude/jsonl_parser.rs`

Claude CLI writes usage-limit errors to **stderr** as plain text. The reset time is an ISO 8601 timestamp embedded in the message, e.g.:
```
Claude AI usage limit reached. Resets at: 2026-03-04T03:00:00.000Z
Error: Usage limit reached; resets at 2026-03-04T11:00:00Z
```

**Step 1: Write the failing test**

Append to `backend/tests/jsonl_parser_test.rs`:
```rust
use ai_kanban_backend::claude::jsonl_parser::extract_rate_limit_reset_at;

#[test]
fn test_extract_reset_at_z_suffix() {
    let line = "Claude AI usage limit reached. Resets at: 2026-03-04T03:00:00.000Z";
    let ts = extract_rate_limit_reset_at(line);
    assert!(ts.is_some());
    let ts = ts.unwrap();
    assert_eq!(ts.year(), 2026);
    assert_eq!(ts.month(), 3);
    assert_eq!(ts.day(), 4);
}

#[test]
fn test_extract_reset_at_no_timestamp_returns_none() {
    let ts = extract_rate_limit_reset_at("Some other error message");
    assert!(ts.is_none());
}

#[test]
fn test_extract_reset_at_no_rate_limit_keyword_returns_none() {
    // ISO timestamp present but no rate-limit context
    let ts = extract_rate_limit_reset_at("Log: 2026-03-04T03:00:00Z processed");
    assert!(ts.is_none());
}
```

**Step 2: Run to confirm FAIL**
```bash
cd backend && cargo test --test jsonl_parser_test test_extract_reset_at 2>&1 | grep -E "FAILED|error"
```
Expected: compile error — function not found.

**Step 3: Implement in `jsonl_parser.rs`**

Add at the bottom of the file:
```rust
use chrono::{DateTime, Utc};

/// Detect a Claude usage-limit message in a stderr line and parse the reset timestamp.
/// Matches patterns like "resets at 2026-03-04T03:00:00.000Z" (case-insensitive).
/// Returns None if the line is not a usage-limit message or timestamp cannot be parsed.
pub fn extract_rate_limit_reset_at(line: &str) -> Option<DateTime<Utc>> {
    let lower = line.to_lowercase();
    // Must contain a rate/usage limit keyword
    if !lower.contains("usage limit") && !lower.contains("rate limit") {
        return None;
    }
    // Find an ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SS with optional ms and Z/offset)
    let re = regex::Regex::new(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})"
    ).ok()?;
    let mat = re.find(line)?;
    DateTime::parse_from_rfc3339(mat.as_str())
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}
```

Add `regex` to `Cargo.toml` dependencies:
```toml
regex = "1"
```

**Step 4: Run to confirm PASS**
```bash
cd backend && cargo test --test jsonl_parser_test test_extract_reset_at 2>&1 | tail -5
```
Expected: 3 tests pass.

**Step 5: Commit**
```bash
git add backend/src/claude/jsonl_parser.rs backend/tests/jsonl_parser_test.rs backend/Cargo.toml backend/Cargo.lock
git commit -m "feat: add extract_rate_limit_reset_at to detect Claude usage limit from stderr"
```

---

## Task 2: `ClaudeEvent::RateLimited` + `ServerMessage::RateLimited`

**Files:**
- Modify: `backend/src/claude/manager.rs` (ClaudeEvent enum)
- Modify: `backend/src/ws/messages.rs` (ServerMessage enum)
- Modify: `backend/src/ws/handler.rs` (forward the event)

**Step 1: Add to `ClaudeEvent` enum** in `manager.rs` after `SessionIdAssigned`:
```rust
RateLimited {
    session_id: String,
    task_id: String,
    stage: String,                        // current task stage at time of limit
    claude_session_id: Option<String>,    // for --resume on retry
    reset_at: chrono::DateTime<chrono::Utc>,
},
```

**Step 2: Add to `ServerMessage` enum** in `ws/messages.rs`:
```rust
#[serde(rename = "rate_limited")]
RateLimited {
    session_id: String,
    task_id: String,
    reset_at: String,   // ISO 8601
},
```

**Step 3: Handle in `ws/handler.rs`**

In the match arm that converts `ClaudeEvent` → `ServerMessage`, add:
```rust
ClaudeEvent::RateLimited { session_id, task_id, reset_at, .. } => {
    let _ = ws_tx.send(ServerMessage::RateLimited {
        session_id,
        task_id,
        reset_at: reset_at.to_rfc3339(),
    });
}
```

**Step 4: Build**
```bash
cd backend && cargo build 2>&1 | grep -E "^error" | head -20
```
Expected: clean build.

**Step 5: Commit**
```bash
git add backend/src/claude/manager.rs backend/src/ws/messages.rs backend/src/ws/handler.rs
git commit -m "feat: add RateLimited event to ClaudeEvent and ServerMessage"
```

---

## Task 3: Detect rate limit in stderr + emit `RateLimited` event from manager

**Files:**
- Modify: `backend/src/claude/manager.rs`

The stderr reader runs in a `tokio::task::spawn_blocking` closure. It needs to pass the detected reset time to the completion handler (which also runs in an async task). Use `Arc<Mutex<Option<DateTime<Utc>>>>`.

**Step 1: Add shared state before spawning stderr/completion tasks**

In `start_session()`, after the stdout_handle spawn and before the stderr_handle spawn, add:
```rust
use std::sync::Mutex as StdMutex;

// Shared: stderr reader sets this if it detects a rate-limit message
let rate_limit_reset: Arc<StdMutex<Option<chrono::DateTime<chrono::Utc>>>> =
    Arc::new(StdMutex::new(None));
let rate_limit_reset_for_stderr = rate_limit_reset.clone();
let rate_limit_reset_for_completion = rate_limit_reset.clone();
```

**Step 2: Update stderr reader to detect rate limit**

Replace the existing stderr spawn_blocking closure body:
```rust
let stderr_handle = tokio::task::spawn_blocking(move || {
    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        if let Ok(text) = line {
            warn!(session_id = %session_id, "stderr: {}", text);

            // Check for usage limit reset timestamp
            if let Some(reset_at) = extract_rate_limit_reset_at(&text) {
                let mut guard = rate_limit_reset_for_stderr.lock().unwrap();
                *guard = Some(reset_at);
            }

            let _ = output_tx.send(ClaudeEvent::Output {
                session_id: session_id.clone(),
                text,
                is_error: true,
            });
        }
    }
});
```

Add `extract_rate_limit_reset_at` to the import at the top of manager.rs:
```rust
use crate::claude::jsonl_parser::{extract_claude_session_id, extract_rate_limit_reset_at, extract_result_text, parse_for_display, parse_jsonl_line};
```

**Step 3: Update completion handler to emit `RateLimited` instead of `failed`**

In the completion task, replace the `final_status` decision block:
```rust
// Check if this was a rate-limit exit
let rate_limit_reset_at = rate_limit_reset_for_completion.lock().unwrap().take();

let final_status = if exit_ok {
    "completed"
} else if rate_limit_reset_at.is_some() {
    "stopped"   // not "failed" — will be retried
} else {
    "failed"
};

let _ = session_repo_for_completion.update(&session_id_for_completion, UpdateSession {
    status: Some(final_status.to_string()),
    ended_at: Some(chrono::Utc::now()),
    error_message: rate_limit_reset_at.as_ref().map(|dt| {
        format!("rate_limited:{}", dt.to_rfc3339())
    }),
    ..Default::default()
}).await;

let _ = output_tx_for_completion.send(ClaudeEvent::SessionStatus {
    session_id: session_id_for_completion.clone(),
    status: final_status.to_string(),
});

// Emit RateLimited event so main.rs can schedule retry
if let Some(reset_at) = rate_limit_reset_at {
    // Fetch current task stage for re-queuing at the right stage
    if let Ok(session) = session_repo_for_completion.find(&session_id_for_completion).await {
        if let Ok(task) = task_repo_for_completion.find(&session.task_id).await {
            let _ = output_tx_for_completion.send(ClaudeEvent::RateLimited {
                session_id: session_id_for_completion.clone(),
                task_id: session.task_id.clone(),
                stage: task.stage.clone(),
                claude_session_id: session.claude_session_id,
                reset_at,
            });
        }
    }
    return; // skip normal success/failure handling below
}
```

**Step 4: Build**
```bash
cd backend && cargo build 2>&1 | grep "^error" | head -20
```
Expected: clean build.

**Step 5: Commit**
```bash
git add backend/src/claude/manager.rs
git commit -m "feat: detect rate limit in stderr, emit RateLimited event instead of failed"
```

---

## Task 4: `SessionQueue::schedule_rate_limit_retry()`

**Files:**
- Modify: `backend/src/claude/queue.rs`

**Step 1: Add the method** to `SessionQueue` (after `get_queued_tasks`):
```rust
/// Called when a session hits a Claude usage limit.
/// Sleeps until `reset_at` then re-enqueues the task using `--resume <claude_session_id>`.
pub async fn schedule_rate_limit_retry(
    self: Arc<Self>,
    task_id: String,
    stage: String,
    claude_session_id: Option<String>,
    reset_at: chrono::DateTime<chrono::Utc>,
) {
    let task_repo = self.task_repo.clone();
    tokio::spawn(async move {
        // Sleep until the rate limit resets (add 5s buffer)
        let now = chrono::Utc::now();
        let wait_secs = (reset_at - now).num_seconds().max(0) as u64 + 5;
        info!(
            task_id = %task_id,
            reset_at = %reset_at,
            wait_secs = wait_secs,
            "Rate limit detected — scheduling retry"
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;

        match task_repo.find(&task_id).await {
            Ok(task) => {
                info!(task_id = %task_id, "Rate limit reset — re-queuing task");
                if let Err(e) = self.enqueue(task, &stage, None, claude_session_id).await {
                    error!(task_id = %task_id, error = %e, "Failed to re-queue rate-limited task");
                }
            }
            Err(e) => error!(task_id = %task_id, error = %e, "Failed to find task for rate-limit retry"),
        }
    });
}
```

**Step 2: Build**
```bash
cd backend && cargo build 2>&1 | grep "^error" | head -10
```

**Step 3: Commit**
```bash
git add backend/src/claude/queue.rs
git commit -m "feat: add schedule_rate_limit_retry to SessionQueue"
```

---

## Task 5: Wire rate limit listener in `main.rs`

**Files:**
- Modify: `backend/src/main.rs`

**Step 1: After creating the queue and manager, add listener**

Add after the `tracing_subscriber::registry()...init()` block:
```rust
// Rate-limit listener: forwards RateLimited events from the manager to the queue
{
    let mut event_rx = claude_manager.subscribe();
    let queue_for_rl = queue.clone();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let ai_kanban_backend::claude::ClaudeEvent::RateLimited {
                task_id, stage, claude_session_id, reset_at, ..
            } = event {
                queue_for_rl.clone().schedule_rate_limit_retry(
                    task_id, stage, claude_session_id, reset_at,
                ).await;
            }
        }
    });
}
```

**Step 2: Build and run a quick sanity check**
```bash
cd backend && cargo build 2>&1 | grep "^error" | head -10
```

**Step 3: Commit**
```bash
git add backend/src/main.rs
git commit -m "feat: wire rate-limit listener in main.rs to auto-resume queued tasks"
```

---

## Task 6: Frontend — show rate limit countdown in live output panel

**Files:**
- Modify: `frontend/src/types/session.ts` (add `rate_limited` WS event type)
- Modify: `frontend/src/components/sessions/live-output-panel.tsx`

**Step 1: Add WS event type** in `frontend/src/types/session.ts`:
```typescript
export interface RateLimitedEvent {
  type: 'rate_limited';
  session_id: string;
  task_id: string;
  reset_at: string; // ISO 8601
}
```

**Step 2: Handle in `live-output-panel.tsx`**

Add state and subscribe after existing `session_id_assigned` subscriber:
```typescript
const [rateLimitResetAt, setRateLimitResetAt] = useState<Date | null>(null);

// Subscribe to rate_limited events
useEffect(() => {
  if (!sessionId) return;
  return subscribe('rate_limited', (data: unknown) => {
    const event = data as RateLimitedEvent;
    if (event.session_id === sessionId) {
      setRateLimitResetAt(new Date(event.reset_at));
    }
  });
}, [sessionId, subscribe]);

// Countdown display — compute inside render
const rateLimitCountdown = rateLimitResetAt
  ? (() => {
      const secsLeft = Math.max(0, Math.round((rateLimitResetAt.getTime() - Date.now()) / 1000));
      if (secsLeft === 0) return 'Resuming now…';
      const h = Math.floor(secsLeft / 3600);
      const m = Math.floor((secsLeft % 3600) / 60);
      const s = secsLeft % 60;
      return h > 0
        ? `Rate limited — resuming in ${h}h ${m}m`
        : `Rate limited — resuming in ${m}m ${s}s`;
    })()
  : null;
```

Add a countdown banner above the log output (in the JSX):
```tsx
{rateLimitResetAt && (
  <div className="px-4 py-2 bg-amber-50 border-b border-amber-200 text-amber-800 text-sm flex items-center gap-2">
    <span>⏳</span>
    <span>{rateLimitCountdown}</span>
    <span className="text-amber-600 text-xs ml-auto">
      Auto-resumes at {rateLimitResetAt.toLocaleTimeString()}
    </span>
  </div>
)}
```

Use `setInterval` (1s) while `rateLimitResetAt` is set to force re-renders for the countdown:
```typescript
useEffect(() => {
  if (!rateLimitResetAt) return;
  const interval = setInterval(() => {
    // Force re-render to update countdown string
    setRateLimitResetAt(prev => prev ? new Date(prev.getTime()) : null);
    // Clear when past reset time + 10s
    if (Date.now() > rateLimitResetAt.getTime() + 10_000) {
      setRateLimitResetAt(null);
    }
  }, 1000);
  return () => clearInterval(interval);
}, [rateLimitResetAt]);
```

**Step 3: Check TypeScript**
```bash
cd frontend && npx tsc --noEmit 2>&1 | grep "error TS" | head -20
```

**Step 4: Commit**
```bash
git add frontend/src/types/session.ts frontend/src/components/sessions/live-output-panel.tsx
git commit -m "feat: show rate limit countdown in live output panel with auto-resume"
```

---

## Task 7: DB migration + `OtelMetric` model

**Files:**
- Create: `backend/migrations/007_otel_metrics.sql`
- Modify: `backend/src/models/mod.rs`
- Create: `backend/src/models/otel_metric.rs`

**Step 1: Create migration**
```sql
-- backend/migrations/007_otel_metrics.sql
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
    -- OTel timestamp (Unix nanoseconds as integer string)
    otel_timestamp    INTEGER NOT NULL,
    created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_task_id       ON otel_metrics(task_id);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_session_id    ON otel_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_metric_name   ON otel_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_otel_metrics_claude_sid    ON otel_metrics(claude_session_id);
```

**Step 2: Create model**
```rust
// backend/src/models/otel_metric.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OtelMetric {
    pub id: i64,
    pub metric_name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub claude_session_id: String,
    pub attributes: String,  // JSON text
    pub otel_timestamp: i64, // Unix nanoseconds
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOtelMetric {
    pub metric_name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub claude_session_id: String,
    pub attributes: serde_json::Value,
    pub otel_timestamp: i64,
}

/// Aggregated dev-activity row for Analytics (ACTO sessions only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevActivityRow {
    pub task_id: String,
    pub task_title: String,
    pub session_id: String,
    pub lines_added: f64,
    pub lines_deleted: f64,
    pub commits: f64,
    pub pull_requests: f64,
    pub active_time_secs: f64,
    pub cost_usd: f64,
}
```

**Step 3: Export from `models/mod.rs`**

Add:
```rust
pub mod otel_metric;
pub use otel_metric::{CreateOtelMetric, DevActivityRow, OtelMetric};
```

**Step 4: Verify migration runs**
```bash
cd backend && cargo build 2>&1 | grep "^error" | head -10
```
(sqlx auto-runs migrations on startup; build confirms types compile.)

**Step 5: Commit**
```bash
git add backend/migrations/007_otel_metrics.sql backend/src/models/otel_metric.rs backend/src/models/mod.rs
git commit -m "feat: add otel_metrics table migration and OtelMetric model"
```

---

## Task 8: `OtelMetricsRepository`

**Files:**
- Create: `backend/src/db/otel_metrics.rs`
- Modify: `backend/src/db/mod.rs`
- Modify: `backend/src/lib.rs` (already exports db)

**Step 1: Write a failing test** in a new file `backend/tests/otel_metrics_test.rs`:
```rust
use ai_kanban_backend::db::{create_pool, OtelMetricsRepository, SessionRepository, TaskRepository};
use ai_kanban_backend::models::{CreateOtelMetric, CreateSession, CreateTask};

async fn setup() -> (OtelMetricsRepository, SessionRepository, TaskRepository) {
    let db_path = format!("/tmp/test-otel-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    (
        OtelMetricsRepository::new(pool.clone()),
        SessionRepository::new(pool.clone()),
        TaskRepository::new(pool.clone()),
    )
}

#[tokio::test]
async fn test_insert_and_query_unaffiliated() {
    let (repo, _, _) = setup().await;
    repo.insert(CreateOtelMetric {
        metric_name: "claude_code.token.usage".to_string(),
        value: 1000.0,
        unit: Some("token".to_string()),
        session_id: None,
        task_id: None,
        claude_session_id: "external-session-abc".to_string(),
        attributes: serde_json::json!({"type": "input"}),
        otel_timestamp: 1709000000000000000,
    }).await.unwrap();

    let rows = repo.dev_activity().await.unwrap();
    // Unaffiliated session should NOT appear in dev_activity
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_dev_activity_correlated_session() {
    let (repo, session_repo, task_repo) = setup().await;

    let task = task_repo.create(CreateTask {
        title: "Dev Task".to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    repo.insert(CreateOtelMetric {
        metric_name: "claude_code.commit.count".to_string(),
        value: 3.0,
        unit: None,
        session_id: Some(session.id.clone()),
        task_id: Some(task.id.clone()),
        claude_session_id: "acto-session-xyz".to_string(),
        attributes: serde_json::json!({}),
        otel_timestamp: 1709000000000000000,
    }).await.unwrap();

    let rows = repo.dev_activity().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].task_title, "Dev Task");
    assert!((rows[0].commits - 3.0).abs() < 0.01);
}

#[tokio::test]
async fn test_correlate_by_claude_session_id() {
    let (repo, session_repo, task_repo) = setup().await;

    let task = task_repo.create(CreateTask {
        title: "Corr Task".to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    // Simulate: OTel data arrives before we can correlate (session_id = None)
    let claude_sid = "known-claude-session-id";
    repo.insert(CreateOtelMetric {
        metric_name: "claude_code.active_time.total".to_string(),
        value: 3600.0,
        unit: Some("s".to_string()),
        session_id: None,
        task_id: None,
        claude_session_id: claude_sid.to_string(),
        attributes: serde_json::json!({}),
        otel_timestamp: 1709000000000000000,
    }).await.unwrap();

    // Now correlate by updating via claude_session_id
    repo.correlate(claude_sid, &session.id, &task.id).await.unwrap();

    let rows = repo.dev_activity().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!((rows[0].active_time_secs - 3600.0).abs() < 0.01);
}
```

**Step 2: Run to confirm FAIL**
```bash
cd backend && cargo test --test otel_metrics_test 2>&1 | grep -E "error\[|FAILED" | head -10
```

**Step 3: Implement the repository**
```rust
// backend/src/db/otel_metrics.rs
use crate::db::create_pool;
use crate::models::{CreateOtelMetric, DevActivityRow, OtelMetric};
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct OtelMetricsRepository {
    pool: SqlitePool,
}

impl OtelMetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, m: CreateOtelMetric) -> Result<OtelMetric> {
        let attrs = serde_json::to_string(&m.attributes)?;
        let row = sqlx::query_as!(
            OtelMetric,
            r#"INSERT INTO otel_metrics
               (metric_name, value, unit, session_id, task_id, claude_session_id, attributes, otel_timestamp)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, metric_name, value, unit, session_id, task_id,
                         claude_session_id, attributes, otel_timestamp,
                         created_at as "created_at: _""#,
            m.metric_name, m.value, m.unit, m.session_id, m.task_id,
            m.claude_session_id, attrs, m.otel_timestamp
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Back-fill session_id/task_id for metrics received before ACTO correlated them.
    pub async fn correlate(&self, claude_session_id: &str, session_id: &str, task_id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE otel_metrics SET session_id = ?, task_id = ?
             WHERE claude_session_id = ? AND session_id IS NULL",
            session_id, task_id, claude_session_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Aggregated dev-activity for ACTO sessions (task_id IS NOT NULL).
    /// One row per (task_id, session_id) pair.
    pub async fn dev_activity(&self) -> Result<Vec<DevActivityRow>> {
        let rows = sqlx::query_as!(
            DevActivityRow,
            r#"SELECT
                 om.task_id    as "task_id!",
                 t.title       as "task_title!",
                 om.session_id as "session_id!",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.lines_of_code.count'
                                    AND json_extract(om.attributes, '$.type') = 'added'
                                   THEN om.value ELSE 0 END), 0) as "lines_added!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.lines_of_code.count'
                                    AND json_extract(om.attributes, '$.type') = 'removed'
                                   THEN om.value ELSE 0 END), 0) as "lines_deleted!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.commit.count'
                                   THEN om.value ELSE 0 END), 0) as "commits!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.pull_request.count'
                                   THEN om.value ELSE 0 END), 0) as "pull_requests!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.active_time.total'
                                   THEN om.value ELSE 0 END), 0) as "active_time_secs!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.cost.usage'
                                   THEN om.value ELSE 0 END), 0) as "cost_usd!: f64"
               FROM otel_metrics om
               JOIN tasks t ON t.id = om.task_id
               WHERE om.task_id IS NOT NULL
               GROUP BY om.task_id, om.session_id, t.title
               ORDER BY MAX(om.created_at) DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
```

**Step 4: Export from `db/mod.rs`**

Add:
```rust
pub mod otel_metrics;
pub use otel_metrics::OtelMetricsRepository;
```

**Step 5: Run tests**
```bash
cd backend && cargo test --test otel_metrics_test 2>&1 | tail -10
```
Expected: 3 pass.

**Step 6: Commit**
```bash
git add backend/src/db/otel_metrics.rs backend/src/db/mod.rs backend/tests/otel_metrics_test.rs
git commit -m "feat: add OtelMetricsRepository with insert, correlate, and dev_activity query"
```

---

## Task 9: OTLP JSON parser

**Files:**
- Create: `backend/src/api/otlp_parser.rs`

OTLP/HTTP JSON format. Each POST body looks like:
```json
{
  "resourceMetrics": [{
    "resource": {
      "attributes": [
        {"key": "session.id", "value": {"stringValue": "abc-123"}},
        {"key": "app.version", "value": {"stringValue": "2.1.63"}}
      ]
    },
    "scopeMetrics": [{
      "metrics": [{
        "name": "claude_code.token.usage",
        "unit": "token",
        "sum": {
          "dataPoints": [{
            "attributes": [{"key": "type", "value": {"stringValue": "input"}}],
            "asInt": 1000,
            "timeUnixNano": "1709000000000000000"
          }]
        }
      }]
    }]
  }]
}
```

**Step 1: Write tests** in a new file (unit tests in the module itself):

**Step 2: Create `otlp_parser.rs`**:
```rust
// backend/src/api/otlp_parser.rs
use crate::models::CreateOtelMetric;
use serde_json::Value;

const TRACKED_METRICS: &[&str] = &[
    "claude_code.token.usage",
    "claude_code.cost.usage",
    "claude_code.session.count",
    "claude_code.lines_of_code.count",
    "claude_code.commit.count",
    "claude_code.pull_request.count",
    "claude_code.active_time.total",
];

/// Parse an OTLP/HTTP JSON metrics payload into CreateOtelMetric records.
/// Only extracts metrics in TRACKED_METRICS.
pub fn parse_otlp_metrics(body: &Value) -> Vec<CreateOtelMetric> {
    let mut results = Vec::new();

    let resource_metrics = match body.get("resourceMetrics").and_then(|v| v.as_array()) {
        Some(rm) => rm,
        None => return results,
    };

    for rm in resource_metrics {
        // Extract resource-level session.id
        let claude_session_id = extract_string_attr(
            rm.get("resource").and_then(|r| r.get("attributes")),
            "session.id",
        )
        .unwrap_or_default();

        // Build resource attributes map
        let resource_attrs = build_attrs(rm.get("resource").and_then(|r| r.get("attributes")));

        let scope_metrics = match rm.get("scopeMetrics").and_then(|v| v.as_array()) {
            Some(sm) => sm,
            None => continue,
        };

        for sm in scope_metrics {
            let metrics = match sm.get("metrics").and_then(|v| v.as_array()) {
                Some(m) => m,
                None => continue,
            };

            for metric in metrics {
                let name = match metric.get("name").and_then(|v| v.as_str()) {
                    Some(n) if TRACKED_METRICS.contains(&n) => n.to_string(),
                    _ => continue,
                };
                let unit = metric.get("unit").and_then(|v| v.as_str()).map(|s| s.to_string());

                // Handle sum and gauge data point arrays
                let data_points = metric
                    .get("sum").and_then(|s| s.get("dataPoints"))
                    .or_else(|| metric.get("gauge").and_then(|g| g.get("dataPoints")))
                    .and_then(|dp| dp.as_array());

                if let Some(dps) = data_points {
                    for dp in dps {
                        let value = dp.get("asInt").and_then(|v| v.as_f64())
                            .or_else(|| dp.get("asDouble").and_then(|v| v.as_f64()))
                            .unwrap_or(0.0);

                        let otel_timestamp = dp.get("timeUnixNano")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<i64>().ok())
                            .or_else(|| dp.get("timeUnixNano").and_then(|v| v.as_i64()))
                            .unwrap_or(0);

                        // Merge resource attrs + data-point attrs
                        let mut attrs = resource_attrs.clone();
                        let dp_attrs = build_attrs(dp.get("attributes"));
                        for (k, v) in dp_attrs.as_object().unwrap_or(&serde_json::Map::new()) {
                            attrs[k] = v.clone();
                        }

                        results.push(CreateOtelMetric {
                            metric_name: name.clone(),
                            value,
                            unit: unit.clone(),
                            session_id: None,   // correlation happens in the handler
                            task_id: None,
                            claude_session_id: claude_session_id.clone(),
                            attributes: attrs,
                            otel_timestamp,
                        });
                    }
                }
            }
        }
    }

    results
}

fn extract_string_attr(attrs: Option<&Value>, key: &str) -> Option<String> {
    attrs?.as_array()?.iter().find_map(|a| {
        if a.get("key")?.as_str()? == key {
            a.get("value")?.get("stringValue")?.as_str().map(|s| s.to_string())
        } else {
            None
        }
    })
}

fn build_attrs(attrs: Option<&Value>) -> Value {
    let mut map = serde_json::Map::new();
    if let Some(arr) = attrs.and_then(|a| a.as_array()) {
        for a in arr {
            if let (Some(key), Some(val)) = (
                a.get("key").and_then(|k| k.as_str()),
                a.get("value"),
            ) {
                let scalar = val.get("stringValue").or_else(|| val.get("intValue"))
                    .or_else(|| val.get("doubleValue")).or_else(|| val.get("boolValue"))
                    .cloned()
                    .unwrap_or(Value::Null);
                map.insert(key.to_string(), scalar);
            }
        }
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_body(metric_name: &str, value: i64, session_id: &str) -> Value {
        serde_json::json!({
            "resourceMetrics": [{
                "resource": {
                    "attributes": [
                        {"key": "session.id", "value": {"stringValue": session_id}}
                    ]
                },
                "scopeMetrics": [{
                    "metrics": [{
                        "name": metric_name,
                        "unit": "1",
                        "sum": {
                            "dataPoints": [{
                                "attributes": [],
                                "asInt": value,
                                "timeUnixNano": "1709000000000000000"
                            }]
                        }
                    }]
                }]
            }]
        })
    }

    #[test]
    fn test_parse_tracked_metric() {
        let body = sample_body("claude_code.commit.count", 3, "sess-abc");
        let results = parse_otlp_metrics(&body);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metric_name, "claude_code.commit.count");
        assert!((results[0].value - 3.0).abs() < 0.01);
        assert_eq!(results[0].claude_session_id, "sess-abc");
    }

    #[test]
    fn test_parse_untracked_metric_ignored() {
        let body = sample_body("some.internal.metric", 99, "sess-abc");
        let results = parse_otlp_metrics(&body);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_data_point_attributes() {
        let body = serde_json::json!({
            "resourceMetrics": [{
                "resource": {"attributes": [
                    {"key": "session.id", "value": {"stringValue": "s1"}}
                ]},
                "scopeMetrics": [{"metrics": [{
                    "name": "claude_code.lines_of_code.count",
                    "sum": {"dataPoints": [{
                        "attributes": [
                            {"key": "type", "value": {"stringValue": "added"}}
                        ],
                        "asInt": 50,
                        "timeUnixNano": "1709000000000000000"
                    }]}
                }]}]
            }]
        });
        let results = parse_otlp_metrics(&body);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attributes["type"], "added");
        assert!((results[0].value - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_body_returns_empty() {
        let results = parse_otlp_metrics(&serde_json::json!({}));
        assert!(results.is_empty());
    }
}
```

**Step 3: Run unit tests**
```bash
cd backend && cargo test otlp_parser 2>&1 | tail -10
```
Expected: 4 pass.

**Step 4: Commit**
```bash
git add backend/src/api/otlp_parser.rs
git commit -m "feat: add OTLP JSON metrics parser (tracked metrics only)"
```

---

## Task 10: OTLP API handler + second Axum server on port 4318

**Files:**
- Create: `backend/src/api/otlp.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/main.rs`

**Step 1: Create `otlp.rs`**
```rust
// backend/src/api/otlp.rs
use crate::api::otlp_parser::parse_otlp_metrics;
use crate::db::{OtelMetricsRepository, SessionRepository};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::Value;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct OtlpState {
    pub otel_repo: OtelMetricsRepository,
    pub session_repo: SessionRepository,
}

/// POST /v1/metrics
/// Accepts OTLP/HTTP JSON metrics payload.
pub async fn receive_metrics(
    State(state): State<OtlpState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let metrics = parse_otlp_metrics(&body);
    let count = metrics.len();
    debug!(count, "Received OTLP metrics batch");

    for mut m in metrics {
        // Attempt to correlate: look up sessions.claude_session_id → session row
        if !m.claude_session_id.is_empty() {
            match state.session_repo.find_by_claude_session_id(&m.claude_session_id).await {
                Ok(Some(session)) => {
                    m.session_id = Some(session.id.clone());
                    m.task_id = Some(session.task_id.clone());
                }
                Ok(None) => {
                    debug!(
                        claude_session_id = %m.claude_session_id,
                        "No ACTO session found for OTel metric — storing unaffiliated"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Failed to look up session for OTel correlation");
                }
            }
        }

        if let Err(e) = state.otel_repo.insert(m).await {
            warn!(error = %e, "Failed to insert OTel metric");
        }
    }

    StatusCode::OK
}

/// POST /v1/logs  (accept and discard for now — future: store as structured events)
pub async fn receive_logs(Json(_body): Json<Value>) -> impl IntoResponse {
    StatusCode::OK
}

pub fn otlp_router(state: OtlpState) -> axum::Router {
    use axum::routing::post;
    axum::Router::new()
        .route("/v1/metrics", post(receive_metrics))
        .route("/v1/logs", post(receive_logs))
        .with_state(state)
}
```

**Step 2: Add `find_by_claude_session_id` to `SessionRepository`**

In `backend/src/db/sessions.rs`, add:
```rust
pub async fn find_by_claude_session_id(&self, claude_session_id: &str) -> Result<Option<Session>> {
    let row = sqlx::query_as!(
        Session,
        r#"SELECT id, task_id, status, started_at as "started_at: _",
                  ended_at as "ended_at: _", last_snapshot_id, error_message, claude_session_id
           FROM sessions WHERE claude_session_id = ? LIMIT 1"#,
        claude_session_id
    )
    .fetch_optional(&self.pool)
    .await?;
    Ok(row)
}
```

**Step 3: Export `OtlpState` and `otlp_router` from `api/mod.rs`**

Add to `backend/src/api/mod.rs`:
```rust
pub mod otlp;
mod otlp_parser;
pub use otlp::{OtlpState, otlp_router};
```

**Step 4: Wire second Axum listener in `main.rs`**

After the existing `AppState` creation, add:
```rust
use ai_kanban_backend::db::OtelMetricsRepository;
use ai_kanban_backend::api::{OtlpState, otlp_router};

let otel_repo = OtelMetricsRepository::new(pool.clone());

// Second server: OTLP receiver on port 4318
let otlp_state = OtlpState {
    otel_repo: otel_repo.clone(),
    session_repo: session_repo.clone(),
};
let otlp_app = otlp_router(otlp_state).layer(
    CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any),
);
let otlp_addr = SocketAddr::from(([0, 0, 0, 0], 4318));
let otlp_listener = tokio::net::TcpListener::bind(otlp_addr).await?;
tracing::info!("OTLP receiver listening on {}", otlp_addr);
tokio::spawn(async move {
    axum::serve(otlp_listener, otlp_app).await.expect("OTLP server failed");
});
```

**Step 5: Set OTEL env vars when spawning Claude** in `manager.rs` `start_session()`:

In the `Command::new("claude")` builder, add after the existing `.args()` calls:
```rust
cmd.env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318")
   .env("OTEL_EXPORTER_OTLP_PROTOCOL", "http/json")
   .env("OTEL_METRICS_EXPORTER", "otlp")
   .env("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "delta");
```

**Step 6: Build**
```bash
cd backend && cargo build 2>&1 | grep "^error" | head -20
```
Expected: clean build.

**Step 7: Quick smoke test** — start the backend and verify port 4318 responds:
```bash
# In one terminal:
cd backend && cargo run 2>&1 &
sleep 3
curl -s -X POST http://localhost:4318/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{"resourceMetrics":[]}' \
  -o /dev/null -w "%{http_code}"
# Expected: 200
```

**Step 8: Commit**
```bash
git add backend/src/api/otlp.rs backend/src/api/mod.rs backend/src/db/sessions.rs backend/src/main.rs backend/src/claude/manager.rs
git commit -m "feat: OTLP/HTTP receiver on port 4318 with session correlation"
```

---

## Task 11: Dev Activity analytics API endpoint

**Files:**
- Modify: `backend/src/api/analytics.rs`
- Modify: `backend/src/api/routes.rs`
- Modify: `backend/src/api/mod.rs` (TaskApiState)

**Step 1: Add `otel_repo` to `TaskApiState`**

In `backend/src/api/mod.rs`, add `otel_repo: OtelMetricsRepository` to `TaskApiState` and update `From<AppState>`.

Check how `AppState` is constructed and add:
```rust
pub struct TaskApiState {
    // ... existing fields ...
    pub otel_repo: OtelMetricsRepository,
}
```

Update `AppState::new()` or `with_*` builder to accept `otel_repo`.

**Step 2: Add `GET /api/analytics/dev-activity` handler** to `analytics.rs`:
```rust
pub async fn dev_activity(
    State(state): State<TaskApiState>,
) -> Result<Json<Vec<DevActivityRow>>, StatusCode> {
    match state.otel_repo.dev_activity().await {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            error!(error = %e, "Failed to get dev activity");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
```

**Step 3: Register route** in `routes.rs`:
```rust
.route("/api/analytics/dev-activity", get(analytics::dev_activity))
```

**Step 4: Build**
```bash
cd backend && cargo build 2>&1 | grep "^error" | head -10
```

**Step 5: Commit**
```bash
git add backend/src/api/analytics.rs backend/src/api/routes.rs backend/src/api/mod.rs
git commit -m "feat: add /api/analytics/dev-activity endpoint backed by otel_metrics"
```

---

## Task 12: Frontend — Dev Activity analytics types, hook, and component

**Files:**
- Modify: `frontend/src/types/analytics.ts`
- Modify: `frontend/src/components/analytics/use-analytics.ts`
- Create: `frontend/src/components/analytics/dev-activity-table.tsx`
- Modify: `frontend/src/app/analytics/page.tsx`

**Step 1: Add type** to `frontend/src/types/analytics.ts`:
```typescript
export interface DevActivityRow {
  task_id: string;
  task_title: string;
  session_id: string;
  lines_added: number;
  lines_deleted: number;
  commits: number;
  pull_requests: number;
  active_time_secs: number;
  cost_usd: number;
}
```

**Step 2: Add hook** to `frontend/src/components/analytics/use-analytics.ts` (or `hooks/use-analytics.ts` — check existing file location):
```typescript
export function useDevActivity() {
  return useQuery({
    queryKey: ['analytics', 'dev-activity'],
    queryFn: () => apiClient<DevActivityRow[]>('/api/analytics/dev-activity'),
    refetchInterval: 30_000,
  });
}
```

**Step 3: Create `dev-activity-table.tsx`**
```tsx
'use client';

import { useDevActivity } from '@/hooks/use-analytics'; // adjust path if needed
import type { DevActivityRow } from '@/types/analytics';

function formatTime(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  return `${(secs / 3600).toFixed(1)}h`;
}

export function DevActivityTable() {
  const { data, isLoading } = useDevActivity();

  if (isLoading) {
    return <div className="h-32 animate-pulse rounded-lg bg-muted" />;
  }

  if (!data || data.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
        No dev activity yet. Data appears here once Claude Code sessions emit OTel metrics.
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted/50">
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Task</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Lines +/-</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Commits</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">PRs</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Active Time</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Cost</th>
          </tr>
        </thead>
        <tbody>
          {data.map((row: DevActivityRow) => (
            <tr key={`${row.task_id}-${row.session_id}`} className="border-b border-border last:border-0 hover:bg-muted/30 transition-colors">
              <td className="px-4 py-3 font-medium truncate max-w-[200px]" title={row.task_title}>
                {row.task_title}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                <span className="text-green-600">+{Math.round(row.lines_added)}</span>
                {' / '}
                <span className="text-red-500">-{Math.round(row.lines_deleted)}</span>
              </td>
              <td className="px-4 py-3 text-right tabular-nums">{Math.round(row.commits)}</td>
              <td className="px-4 py-3 text-right tabular-nums">{Math.round(row.pull_requests)}</td>
              <td className="px-4 py-3 text-right tabular-nums text-muted-foreground">
                {formatTime(row.active_time_secs)}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                {row.cost_usd > 0 ? `$${row.cost_usd.toFixed(4)}` : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

**Step 4: Add section to `analytics/page.tsx`**

After the closing `</section>` of "Agent Intelligence", add:
```tsx
import { DevActivityTable } from '@/components/analytics/dev-activity-table';

// In the JSX:
<section className="space-y-4">
  <div>
    <h2 className="text-base font-semibold text-foreground">Dev Activity</h2>
    <p className="text-sm text-muted-foreground">
      Commits, lines changed, and active time per session — sourced from Claude Code telemetry
    </p>
  </div>
  <DevActivityTable />
</section>
```

**Step 5: Check TypeScript**
```bash
cd frontend && npx tsc --noEmit 2>&1 | grep "error TS" | head -20
```

**Step 6: Commit**
```bash
git add frontend/src/types/analytics.ts frontend/src/hooks/use-analytics.ts \
         frontend/src/components/analytics/dev-activity-table.tsx \
         frontend/src/app/analytics/page.tsx
git commit -m "feat: add Dev Activity section to Analytics page backed by OTel metrics"
```

---

## Task 13: Final verification

**Step 1: Run all backend tests**
```bash
cd backend && cargo test 2>&1 | grep -E "test result|FAILED"
```
Expected: all green, 0 failures.

**Step 2: TypeScript check**
```bash
cd frontend && npx tsc --noEmit 2>&1 | grep "error TS" | head -20
```
Expected: no errors.

**Step 3: End-to-end smoke test**

Start the backend, then send a sample OTLP payload:
```bash
curl -s -X POST http://localhost:4318/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "resourceMetrics": [{
      "resource": {
        "attributes": [
          {"key": "session.id", "value": {"stringValue": "test-sid-123"}}
        ]
      },
      "scopeMetrics": [{
        "metrics": [{
          "name": "claude_code.commit.count",
          "unit": "1",
          "sum": {
            "dataPoints": [{
              "attributes": [],
              "asInt": 2,
              "timeUnixNano": "1709000000000000000"
            }]
          }
        }]
      }]
    }]
  }' \
  -w "\nHTTP %{http_code}\n"
```
Expected: `HTTP 200`

**Step 4: Final commit if any loose files**
```bash
git status && git add -A && git commit -m "chore: final cleanup and verification"
```
