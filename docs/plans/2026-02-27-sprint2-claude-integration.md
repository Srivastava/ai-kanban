# Sprint 2: Claude Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate Claude CLI process spawning with session management, output parsing, and concurrency control.

**Architecture:** ClaudeManager spawns Claude processes with stage-appropriate prompts. SessionQueue enforces max 3 concurrent sessions. Output is parsed in real-time and broadcast via WebSocket.

**Tech Stack:** Rust async, tokio::process, regex for parsing, broadcast channels

---

## Task 1: Create Session Model and Repository

**Files:**
- Create: `backend/src/models/session.rs`
- Modify: `backend/src/models/mod.rs`
- Create: `backend/src/db/sessions.rs`
- Modify: `backend/src/db/mod.rs`

**Step 1: Create Session model**

Create `backend/src/models/session.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,                  // Claude session ID
    pub task_id: String,
    pub status: SessionStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_snapshot_id: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum SessionStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
                SessionStatus::Pending => "pending",
                SessionStatus::Running => "running",
                SessionStatus::Paused => "paused",
                SessionStatus::Completed => "completed",
                SessionStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(SessionStatus::Pending),
            "running" => Some(SessionStatus::Running),
            "paused" => Some(SessionStatus::Paused),
            "completed" => Some(SessionStatus::Completed),
            "failed" => Some(SessionStatus::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSession {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSession {
    pub status: Option<SessionStatus>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_snapshot_id: Option<String>,
    pub error_message: Option<String>,
}
```

**Step 2: Update models/mod.rs**

Add `mod session;` and `pub use session::*;`

**Step 3: Create SessionRepository**

Create `backend/src/db/sessions.rs`:

```rust
use crate::models::{CreateSession, Session, SessionStatus, UpdateSession};
use anyhow::{anyhow, Result};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SessionRepository {
    pool: SqlitePool,
}

impl SessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, create: CreateSession) -> Result<Session> {
        let now = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, task_id, status, started_at)
            VALUES (?, ?, 'pending', ?)
            "#,
        )
        .bind(&id)
        .bind(&create.task_id)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Session {
            id,
            task_id: create.task_id,
            status: SessionStatus::Pending,
            started_at: now,
            ended_at: None,
            last_snapshot_id: None,
            error_message: None,
        })
    }

    pub async fn find(&self, id: &str) -> Result<Session> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("Session not found: {}", id))
    }

    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE task_id = ? ORDER BY started_at DESC"
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(&self, id: &str, update: UpdateSession) -> Result<Session> {
        let mut session = self.find(id).await?;

        if let Some(status) = update.status {
            session.status = status;
        }
        if let Some(ended_at) = update.ended_at {
            session.ended_at = Some(ended_at);
        }
        if let Some(snapshot_id) = update.last_snapshot_id {
            session.last_snapshot_id = Some(snapshot_id);
        }
        if let Some(error) = update.error_message {
            session.error_message = Some(error);
        }

        sqlx::query(
            r#"
            UPDATE sessions
            SET status = ?, ended_at = ?, last_snapshot_id = ?, error_message = ?
            WHERE id = ?
            "#,
        )
        .bind(session.status.as_str())
        .bind(session.ended_at.map(|t| t.to_rfc3339()))
        .bind(&session.last_snapshot_id)
        .bind(&session.error_message)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(session)
    }

    pub async fn list_active(&self) -> Result<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE status = 'running' ORDER BY started_at ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }
}
```

**Step 4: Update db/mod.rs**

Add `pub mod sessions;` and `pub use sessions::SessionRepository;`

**Step 5: Verify compilation**

Run: `cd backend && cargo build`

**Step 6: Commit**

```bash
git add backend/src/
git commit -m "feat(db): add Session model and repository

- Session model with status enum
- SessionRepository with CRUD operations
- List active sessions query"
```

---

## Task 2: Create Claude Manager

**Files:**
- Create: `backend/src/claude/mod.rs`
- Create: `backend/src/claude/manager.rs`
- Create: `backend/src/claude/prompts.rs`

**Step 1: Create claude module structure**

Create `backend/src/claude/mod.rs`:

```rust
mod manager;
mod prompts;

pub use manager::ClaudeManager;
pub use prompts::build_prompt;
```

**Step 2: Create prompts module**

Create `backend/src/claude/prompts.rs`:

```rust
use crate::models::Stage;

pub fn build_prompt(task_title: &str, task_description: Option<&str>, stage: &Stage) -> String {
    let stage_instructions = match stage {
        Stage::Planning => {
            "You are in PLANNING mode. Analyze the task and create a detailed implementation plan.
            Do NOT make any code changes. Focus on:
            1. Understanding the requirements
            2. Identifying affected files
            3. Creating a step-by-step plan
            Output your plan in a clear, structured format."
        }
        Stage::InProgress => {
            "You are in IN_PROGRESS mode. Implement the task according to the plan.
            Make code changes as needed. Follow TDD principles:
            1. Write failing tests first
            2. Implement minimal code to pass
            3. Refactor if needed"
        }
        Stage::Review => {
            "You are in REVIEW mode. Review your implementation:
            1. Check for bugs and edge cases
            2. Verify test coverage
            3. Look for improvements
            Do NOT make changes unless you find critical issues."
        }
        _ => "Complete the task as appropriate for the current stage."
    };

    let description_section = task_description
        .map(|d| format!("\n\nTask Description:\n{}", d))
        .unwrap_or_default();

    format!(
        r#"# Task: {}

{}{}

## Instructions
- Work on this task in the context of the current project
- Use the available tools (Read, Write, Edit, Bash, Glob, Grep) as needed
- Report your progress clearly
- When complete, summarize what was done"#,
        task_title,
        stage_instructions,
        description_section
    )
}
```

**Step 3: Create Claude Manager**

Create `backend/src/claude/manager.rs`:

```rust
use crate::claude::prompts::build_prompt;
use crate::db::SessionRepository;
use crate::models::{Session, SessionStatus, Stage, Task};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, instrument, warn};

/// Output from a Claude session
#[derive(Debug, Clone)]
pub struct SessionOutput {
    pub session_id: String,
    pub line: String,
    pub is_error: bool,
}

/// A running Claude session
pub struct ClaudeSession {
    pub session: Session,
    pub child: Child,
    pub task: Task,
}

/// Manages Claude CLI processes
pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, ClaudeSession>>>,
    output_tx: broadcast::Sender<SessionOutput>,
    session_repo: SessionRepository,
}

impl ClaudeManager {
    pub fn new(session_repo: SessionRepository) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            session_repo,
        }
    }

    /// Subscribe to session output
    pub fn subscribe(&self) -> broadcast::Receiver<SessionOutput> {
        self.output_tx.subscribe()
    }

    /// Start a new Claude session for a task
    #[instrument(skip(self, task), fields(task_id = %task.id))]
    pub async fn start_session(&self, task: Task, stage: Stage) -> Result<String> {
        // Create session record
        let session = self.session_repo
            .create(crate::models::CreateSession {
                task_id: task.id.clone(),
            })
            .await?;

        info!(session_id = %session.id, task_id = %task.id, "Starting Claude session");

        // Build prompt based on stage
        let prompt = build_prompt(
            &task.title,
            task.description.as_deref(),
            &stage,
        );

        // Spawn Claude process
        let mut child = Command::new("claude")
            .arg("--print")
            .arg("--continue")
            .arg(&session.id)
            .current_dir(&task.project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("CLAUDE_SESSION_ID", &session.id)
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn Claude: {}", e))?;

        // Take ownership of pipes before moving child
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session.id.clone(), ClaudeSession {
                session: session.clone(),
                child,
                task,
            });
        }

        // Update status to running
        self.session_repo
            .update(&session.id, crate::models::UpdateSession {
                status: Some(SessionStatus::Running),
                ..Default::default()
            })
            .await?;

        // Spawn output reader tasks
        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        debug!(session_id = %session_id, "stdout: {}", text);
                        let _ = output_tx.send(SessionOutput {
                            session_id: session_id.clone(),
                            line: text,
                            is_error: false,
                        });
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "stdout read error: {}", e);
                        break;
                    }
                }
            }
        });

        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        warn!(session_id = %session_id, "stderr: {}", text);
                        let _ = output_tx.send(SessionOutput {
                            session_id: session_id.clone(),
                            line: text,
                            is_error: true,
                        });
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "stderr read error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(session.id)
    }

    /// Stop a running session
    #[instrument(skip(self))]
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;

        if let Some(mut claude_session) = sessions.remove(session_id) {
            info!(session_id = %session_id, "Stopping Claude session");

            // Kill the process
            let _ = claude_session.child.kill();

            // Update status
            self.session_repo
                .update(session_id, crate::models::UpdateSession {
                    status: Some(SessionStatus::Paused),
                    ..Default::default()
                })
                .await?;
        }

        Ok(())
    }

    /// Get active session count
    pub async fn active_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    /// Check if a session is active
    pub async fn is_active(&self, session_id: &str) -> bool {
        self.active_sessions.read().await.contains_key(session_id)
    }
}
```

**Step 4: Update lib.rs**

Add `pub mod claude;`

**Step 5: Add dependencies**

Add to `Cargo.toml`:
```toml
tokio = { version = "1", features = ["full", "process"] }
```

**Step 6: Verify compilation**

Run: `cd backend && cargo build`

**Step 7: Commit**

```bash
git add backend/src/
git commit -m "feat(claude): add ClaudeManager for process spawning

- Stage-appropriate prompts
- Process spawning with stdout/stderr capture
- Broadcast channel for output
- Session lifecycle management"
```

---

## Task 3: Create Session Queue (Concurrency Control)

**Files:**
- Create: `backend/src/claude/queue.rs`
- Modify: `backend/src/claude/mod.rs`

**Step 1: Create session queue**

Create `backend/src/claude/queue.rs`:

```rust
use crate::claude::ClaudeManager;
use crate::db::TaskRepository;
use crate::models::{SessionStatus, Stage, Task};
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, instrument, warn};

/// A task waiting to be processed
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task: Task,
    pub stage: Stage,
    pub queued_at: chrono::DateTime<chrono::Utc>,
}

/// Manages concurrent Claude sessions (max 3)
pub struct SessionQueue {
    max_concurrent: usize,
    manager: Arc<ClaudeManager>,
    task_repo: TaskRepository,
    pending: Arc<Mutex<VecDeque<QueuedTask>>>,
}

impl SessionQueue {
    pub fn new(manager: Arc<ClaudeManager>, task_repo: TaskRepository) -> Self {
        Self {
            max_concurrent: 3,
            manager,
            task_repo,
            pending: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Queue a task for processing
    #[instrument(skip(self, task))]
    pub async fn enqueue(&self, task: Task, stage: Stage) -> Result<()> {
        let active_count = self.manager.active_count().await;

        if active_count < self.max_concurrent {
            // Start immediately
            info!(task_id = %task.id, "Starting task immediately");
            self.manager.start_session(task, stage).await?;
        } else {
            // Add to queue
            info!(task_id = %task.id, "Queuing task ({} active)", active_count);
            let mut pending = self.pending.lock().await;
            pending.push_back(QueuedTask {
                task,
                stage,
                queued_at: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// Called when a session completes - start next queued task
    #[instrument(skip(self))]
    pub async fn on_session_complete(&self, session_id: &str) -> Result<()> {
        info!(session_id = %session_id, "Session completed, checking queue");

        let mut pending = self.pending.lock().await;

        if let Some(queued) = pending.pop_front() {
            info!(task_id = %queued.task.id, "Starting queued task");
            drop(pending); // Release lock before starting

            self.manager.start_session(queued.task, queued.stage).await?;
        }

        Ok(())
    }

    /// Get queue length
    pub async fn queue_length(&self) -> usize {
        self.pending.lock().await.len()
    }

    /// Get all queued tasks
    pub async fn get_queued_tasks(&self) -> Vec<QueuedTask> {
        self.pending.lock().await.iter().cloned().collect()
    }

    /// Remove a task from the queue
    pub async fn dequeue(&self, task_id: &str) -> bool {
        let mut pending = self.pending.lock().await;
        let initial_len = pending.len();
        pending.retain(|qt| qt.task.id != task_id);
        pending.len() != initial_len
    }
}
```

**Step 2: Update claude/mod.rs**

```rust
mod manager;
mod prompts;
mod queue;

pub use manager::{ClaudeManager, SessionOutput};
pub use prompts::build_prompt;
pub use queue::{QueuedTask, SessionQueue};
```

**Step 3: Verify compilation**

Run: `cd backend && cargo build`

**Step 4: Commit**

```bash
git add backend/src/
git commit -m "feat(claude): add SessionQueue for concurrency control

- Max 3 concurrent sessions
- FIFO queue for pending tasks
- Auto-start next task on completion"
```

---

## Task 4: Add Session REST API Endpoints

**Files:**
- Create: `backend/src/api/sessions.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/api/routes.rs`

**Step 1: Create sessions API handlers**

Create `backend/src/api/sessions.rs`:

```rust
use crate::api::AppState;
use crate::claude::{QueuedTask, SessionQueue};
use crate::models::Stage;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct SessionApiState {
    pub queue: Arc<SessionQueue>,
}

pub fn session_routes() -> Router<SessionApiState> {
    Router::new()
        .route("/", get(list_sessions))
        .route("/:id", get(get_session))
        .route("/:id/stop", post(stop_session))
        .route("/queue", get(get_queue))
}

#[derive(Serialize)]
struct SessionInfo {
    id: String,
    task_id: String,
    status: String,
    started_at: String,
}

#[derive(Serialize)]
struct QueueInfo {
    position: usize,
    task_id: String,
    task_title: String,
    stage: String,
    queued_at: String,
}

async fn list_sessions(
    State(state): State<SessionApiState>,
) -> impl IntoResponse {
    // Return queue info since active sessions are managed by ClaudeManager
    let queued = state.queue.get_queued_tasks().await;
    let queue_info: Vec<QueueInfo> = queued
        .into_iter()
        .enumerate()
        .map(|(i, qt)| QueueInfo {
            position: i,
            task_id: qt.task.id,
            task_title: qt.task.title,
            stage: qt.stage.as_str().to_string(),
            queued_at: qt.queued_at.to_rfc3339(),
        })
        .collect();

    Json(queue_info).into_response()
}

async fn get_session(
    State(state): State<SessionApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Check if session is active
    if state.queue.manager.is_active(&id).await {
        Json(serde_json::json!({
            "id": id,
            "status": "running"
        })).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        ).into_response()
    }
}

async fn stop_session(
    State(state): State<SessionApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.queue.manager.stop_session(&id).await {
        Ok(()) => Json(serde_json::json!({ "status": "stopped" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

async fn get_queue(State(state): State<SessionApiState>) -> impl IntoResponse {
    let queued = state.queue.get_queued_tasks().await;
    let queue_info: Vec<QueueInfo> = queued
        .into_iter()
        .enumerate()
        .map(|(i, qt)| QueueInfo {
            position: i,
            task_id: qt.task.id,
            task_title: qt.task.title,
            stage: qt.stage.as_str().to_string(),
            queued_at: qt.queued_at.to_rfc3339(),
        })
        .collect();

    Json(queue_info)
}
```

**Step 2: Update api/mod.rs**

Add session state to AppState:

```rust
use crate::claude::SessionQueue;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub tasks: TaskRepository,
    pub logs: LogRepository,
    pub sessions: SessionRepository,
    pub queue: Option<Arc<SessionQueue>>,
}
```

**Step 3: Update api/routes.rs**

Add session routes to router.

**Step 4: Update main.rs**

Initialize ClaudeManager and SessionQueue.

**Step 5: Verify compilation**

Run: `cd backend && cargo build`

**Step 6: Commit**

```bash
git add backend/src/
git commit -m "feat(api): add session REST API endpoints

- GET /api/sessions - list active/queued sessions
- GET /api/sessions/:id - get session status
- POST /api/sessions/:id/stop - stop session
- GET /api/sessions/queue - get pending queue"
```

---

## Task 5: Add Session Tests

**Files:**
- Create: `backend/tests/session_test.rs`

**Step 1: Create session tests**

Create tests for:
- SessionRepository CRUD operations
- Session status transitions
- Queue enqueue/dequeue
- Concurrency limit enforcement

**Step 2: Run tests**

Run: `cd backend && cargo test`

**Step 3: Commit**

```bash
git add backend/tests/
git commit -m "test: add session and queue tests

- SessionRepository CRUD tests
- Session status transition tests
- Queue concurrency limit tests"
```

---

## Summary

Sprint 2 adds Claude CLI integration:

- **ClaudeManager** - Spawns Claude processes with stage-appropriate prompts
- **SessionQueue** - Enforces max 3 concurrent sessions
- **Session API** - REST endpoints for session management
- **Real-time output** - Broadcast via channel (WebSocket integration in Sprint 3)

**Next Sprint:** Real-time Layer (WebSocket server, message broadcasting)
