# Interactive Claude Sessions Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Claude sessions interactive via comment-driven iteration, stream live output to the UI, fix session lifecycle (stop on delete/done), and polish comments + Kanban.

**Architecture:** The backend captures Claude's final JSONL `result` text and posts it as a `author="claude"` comment. A "Continue Session" button on the task detail passes full comment history as conversation context. Live output streams via WebSocket with per-client session filtering. Session lifecycle is cleaned up: auto-stop on task delete or move-to-Done.

**Tech Stack:** Rust/Axum, SQLite/SQLx, tokio broadcast, Next.js 16, React Query, WebSocket context, @dnd-kit

---

### Task 1: Add `--dangerously-skip-permissions` and wire `comment_repo` + `task_repo` into ClaudeManager

**Why:** Without `--dangerously-skip-permissions`, Claude in `--print` mode responds with text but doesn't execute tools (won't create files, run commands, etc.). Manager also needs `CommentRepository` to post Claude's result as a comment and `TaskRepository` to link `session_id` back to the task.

**Files:**
- Modify: `backend/src/claude/manager.rs`
- Modify: `backend/src/main.rs`

**Step 1: Add repos to ClaudeManager struct**

In `manager.rs`, replace the struct and `new()`:

```rust
use crate::db::{CommentRepository, SessionMetricsRepository, SessionRepository, TaskRepository, TokenEventRepository};
use crate::models::{CreateComment, CreateTokenEvent, Session, Task, UpdateSession, UpdateTask};

pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,
    output_tx: broadcast::Sender<SessionOutput>,
    session_repo: SessionRepository,
    token_event_repo: TokenEventRepository,
    session_metrics_repo: SessionMetricsRepository,
    comment_repo: CommentRepository,
    task_repo: TaskRepository,
}

impl ClaudeManager {
    pub fn new(
        session_repo: SessionRepository,
        token_event_repo: TokenEventRepository,
        session_metrics_repo: SessionMetricsRepository,
        comment_repo: CommentRepository,
        task_repo: TaskRepository,
    ) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            session_repo,
            token_event_repo,
            session_metrics_repo,
            comment_repo,
            task_repo,
        }
    }
```

**Step 2: Add `--dangerously-skip-permissions` to the spawn command**

In `start_session`, in the `Command::new` block, add the flag after `--verbose`:

```rust
let mut child = Command::new(&claude_bin)
    .arg("--print")
    .arg("--verbose")
    .arg("--dangerously-skip-permissions")   // ADD THIS
    .arg("--output-format").arg("stream-json")
    .arg(&prompt)
    .current_dir(&project_path)
    .env_remove("CLAUDECODE")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| anyhow!("Failed to spawn Claude (bin={}): {}", claude_bin, e))?;
```

**Step 3: Add `get_active_session_for_task` method**

```rust
pub async fn get_active_session_for_task(&self, task_id: &str) -> Option<String> {
    let sessions = self.active_sessions.read().await;
    sessions.iter()
        .find(|(_, rs)| rs.task.id == task_id)
        .map(|(session_id, _)| session_id.clone())
}
```

**Step 4: Update `main.rs` to pass new args**

```rust
let claude_manager = Arc::new(ClaudeManager::new(
    session_repo.clone(),
    token_event_repo.clone(),
    session_metrics_repo.clone(),
    comment_repo.clone(),   // ADD
    task_repo.clone(),      // ADD
));
```

**Step 5: Expose on SessionQueue**

In `queue.rs`, add:
```rust
pub async fn get_active_session_for_task(&self, task_id: &str) -> Option<String> {
    self.manager.get_active_session_for_task(task_id).await
}
```

**Step 6: Verify it compiles**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "^error"
```
Expected: no errors.

**Step 7: Commit**
```bash
git add backend/src/claude/manager.rs backend/src/main.rs backend/src/claude/queue.rs
git commit -m "feat: add --dangerously-skip-permissions, wire comment/task repos into ClaudeManager"
```

---

### Task 2: Update `task.session_id` on session start + expose real session status via API

**Why:** `task.session_id` is currently never written after session creation, so the frontend always sees `null`. The `GET /api/sessions/{id}` only returns "running" for in-memory sessions; completed/failed sessions return 404.

**Files:**
- Modify: `backend/src/claude/manager.rs`
- Modify: `backend/src/api/sessions.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/models/task.rs`
- Modify: `backend/src/db/tasks.rs`

**Step 1: Add `session_id` to UpdateTask model**

In `backend/src/models/task.rs`, find `UpdateTask` struct and add:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub context: Option<String>,
    pub stage: Option<String>,
    pub priority: Option<i32>,
    pub session_id: Option<String>,   // ADD THIS
}
```

**Step 2: Update the SQL in `db/tasks.rs`**

Find the `update` method. The query already has `session_id` in some form. Make sure it handles `session_id`:

```rust
pub async fn update(&self, id: &str, update: UpdateTask) -> Result<Task> {
    let task = sqlx::query_as!(
        Task,
        r#"UPDATE tasks SET
            title = COALESCE($1, title),
            description = COALESCE($2, description),
            context = COALESCE($3, context),
            stage = COALESCE($4, stage),
            priority = COALESCE($5, priority),
            session_id = COALESCE($6, session_id),
            updated_at = datetime('now')
        WHERE id = $7
        RETURNING *"#,
        update.title,
        update.description,
        update.context,
        update.stage,
        update.priority,
        update.session_id,
        id
    )
    .fetch_one(&self.pool)
    .await?;
    Ok(task)
}
```

**Step 3: Update task.session_id in manager.rs after session starts**

At the end of `start_session`, after `self.session_repo.update(...)` sets status to "running", add:

```rust
// Link session to task
let task_repo_link = self.task_repo.clone();
let task_id_link = task.id.clone();
let session_id_link = session.id.clone();
tokio::spawn(async move {
    let _ = task_repo_link.update(&task_id_link, UpdateTask {
        session_id: Some(session_id_link),
        ..Default::default()
    }).await;
});
```

**Step 4: Fix `GET /api/sessions/{id}` to return real DB data**

`SessionApiState` needs a `session_repo`. In `api/mod.rs`, change:

```rust
#[derive(Clone)]
pub struct SessionApiState {
    pub queue: Arc<SessionQueue>,
    pub session_repo: SessionRepository,   // ADD
}

impl From<AppState> for SessionApiState {
    fn from(state: AppState) -> Self {
        SessionApiState {
            queue: state.queue.expect("SessionQueue not initialized"),
            session_repo: state.sessions,   // ADD
        }
    }
}
```

In `api/sessions.rs`, rewrite `get_session`:

```rust
#[instrument(skip(state))]
async fn get_session(
    State(state): State<SessionApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.session_repo.find(&id).await {
        Ok(session) => Json(session).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        ).into_response(),
    }
}
```

**Step 5: Verify compile**
```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "^error"
```

**Step 6: Commit**
```bash
git add backend/src/claude/manager.rs backend/src/api/sessions.rs backend/src/api/mod.rs backend/src/models/task.rs backend/src/db/tasks.rs
git commit -m "feat: link session_id to task on start, fix GET /api/sessions/:id"
```

---

### Task 3: Stop active session on task delete and on move to Done

**Files:**
- Modify: `backend/src/api/tasks.rs`

**Step 1: Add session-stop helper to delete_task**

In `delete_task`, before `state.repo.delete(&id)`:

```rust
// Stop any running Claude process for this task
if let Some(queue) = &state.queue {
    if let Some(session_id) = queue.get_active_session_for_task(&id).await {
        let _ = queue.stop_session(&session_id).await;
    }
}
```

**Step 2: Add session-stop to move_task when stage is "done"**

In `move_task`, after the successful `move_to_stage` call:

```rust
// Auto-stop session when task moves to Done
if body.stage == "done" {
    if let Some(queue) = &state.queue {
        if let Some(session_id) = queue.get_active_session_for_task(&id).await {
            let _ = queue.stop_session(&session_id).await;
        }
    }
}
```

**Step 3: Verify compile**
```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "^error"
```

**Step 4: Commit**
```bash
git add backend/src/api/tasks.rs
git commit -m "fix: stop active session on task delete and on move to Done"
```

---

### Task 4: Capture Claude's result text and post as `author="claude"` comment

**Why:** When Claude finishes, its final text response is in the JSONL stream as `{"type":"result","subtype":"success","result":"..."}`. We extract this and create a comment so the user can see Claude's output and reply.

**Files:**
- Modify: `backend/src/claude/manager.rs`
- Modify: `backend/src/claude/jsonl_parser.rs`

**Step 1: Add `extract_result_text` to jsonl_parser.rs**

Add this function at the end of `jsonl_parser.rs`:

```rust
/// Extract the final result text from a Claude result line.
/// Returns Some(text) for {"type":"result","subtype":"success","result":"..."}
pub fn extract_result_text(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "result" {
        return None;
    }
    value.get("result")?.as_str().map(|s| s.to_string())
}
```

**Step 2: Add result_text accumulation in the stdout reader**

In manager.rs, the stdout `spawn_blocking` block needs to accumulate the result text:

Change the stdout reader to capture a result:

```rust
let stdout_handle = tokio::task::spawn_blocking(move || {
    let reader = BufReader::new(stdout);
    let mut sequence_no: i64 = 0;
    let mut result_text: Option<String> = None;  // ADD

    for line in reader.lines() {
        if let Ok(text) = line {
            debug!(session_id = %session_id, "stdout: {}", text);

            // Capture Claude's final result text
            if let Some(r) = parse_result_text(&text) {   // ADD
                result_text = Some(r);
            }

            // existing JSONL token parsing...
            if let Some(parsed) = parse_jsonl_line(&text) {
                // ...existing token event code...
            }

            let _ = output_tx.send(SessionOutput { ... });
        }
    }
    result_text  // RETURN result_text from the closure
});
```

Add the import at top of manager.rs:
```rust
use crate::claude::jsonl_parser::{extract_result_text as parse_result_text, parse_jsonl_line};
```

**Step 3: Use result_text in the completion task**

In the completion `tokio::spawn` block, after waiting on the child:

```rust
// Get the result text that the stdout thread captured
let result_text = match stdout_handle.await {
    Ok(text) => text,   // stdout_handle now returns Option<String>
    Err(_) => None,
};
let _ = stderr_handle.await;
```

Then after `session_repo_for_completion.update(...)`, post the comment if we have result text:

```rust
// Post Claude's response as a comment
if let Some(text) = result_text {
    if !text.is_empty() {
        let task_id_for_comment = {
            let sessions = active_sessions_for_completion.read().await;
            // task was already removed above, get task_id from session record
            drop(sessions);
            // fetch from DB
            session_repo_for_completion.find(&session_id_for_completion).await
                .ok().map(|s| s.task_id)
        };
        if let Some(task_id) = task_id_for_comment {
            use crate::models::CreateComment;
            let _ = comment_repo_for_completion.create(
                &task_id,
                "claude",
                CreateComment { content: text, parent_id: None },
            ).await;
            info!(session_id = %session_id_for_completion, "Posted Claude result as comment");
        }
    }
}
```

Note: you need to clone `comment_repo` and `session_repo` for the completion closure. Add clones before the `tokio::spawn`:

```rust
let comment_repo_for_completion = self.comment_repo.clone();
let session_repo_for_completion2 = self.session_repo.clone(); // for comment task_id lookup
```

**Step 4: Verify compile**
```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "^error"
```

**Step 5: Commit**
```bash
git add backend/src/claude/manager.rs backend/src/claude/jsonl_parser.rs
git commit -m "feat: capture Claude result text and post as claude comment on session complete"
```

---

### Task 5: Add `continue_session` endpoint + build prompt from comment history

**Why:** "Continue Session" needs to include the full comment thread as conversation context so Claude has memory of the prior exchange.

**Files:**
- Modify: `backend/src/claude/prompts.rs`
- Modify: `backend/src/claude/manager.rs` (add `conversation_context` param)
- Modify: `backend/src/claude/queue.rs` (pass through)
- Modify: `backend/src/api/tasks.rs` (new route)
- Modify: `backend/src/api/mod.rs` (add comment_repo to TaskApiState)

**Step 1: Add conversation_context to build_prompt**

In `prompts.rs`:

```rust
pub fn build_prompt(
    task_title: &str,
    task_description: Option<&str>,
    stage: &str,
    conversation_context: Option<&str>,  // ADD
) -> String {
    let stage_instructions = match stage {
        "planning" => "You are in PLANNING mode. Analyze the task and create a detailed implementation plan. Do NOT make any code changes.",
        "in_progress" => "You are in IN_PROGRESS mode. Implement the task according to the plan. Make code changes as needed.",
        "review" => "You are in REVIEW mode. Review your implementation for bugs and improvements.",
        _ => "Complete the task as appropriate for the current stage.",
    };

    let description_section = task_description
        .map(|d| format!("\n\nTask Description:\n{}", d))
        .unwrap_or_default();

    let conversation_section = conversation_context
        .map(|c| format!("\n\n## Conversation History\n{}", c))
        .unwrap_or_default();

    format!(
        "# Task: {}\n\n{}{}{}\n\n## Instructions\n- Work on this task in the project context\n- Use available tools as needed\n- Report progress clearly",
        task_title, stage_instructions, description_section, conversation_section
    )
}
```

**Step 2: Update manager.rs to pass conversation_context to build_prompt**

Change `start_session` signature:
```rust
pub async fn start_session(
    &self,
    task: Task,
    stage: &str,
    conversation_context: Option<String>,  // ADD
) -> Result<String>
```

Update the `build_prompt` call:
```rust
let prompt = build_prompt(
    &task.title,
    task.description.as_deref(),
    stage,
    conversation_context.as_deref(),  // ADD
);
```

**Step 3: Update queue.rs `enqueue` to pass through**

```rust
pub async fn enqueue(
    &self,
    task: Task,
    stage: String,
    conversation_context: Option<String>,  // ADD
) -> Result<()> {
    let active_count = self.manager.active_count().await;
    if active_count < self.max_concurrent {
        info!(task_id = %task.id, "Starting task immediately");
        self.manager.start_session(task, &stage, conversation_context).await?;
    } else {
        info!(task_id = %task.id, "Queuing task ({} active)", active_count);
        let mut pending = self.pending.lock().await;
        pending.push_back(QueuedTask {
            task,
            stage,
            conversation_context,  // ADD this field to QueuedTask too
            queued_at: chrono::Utc::now(),
        });
    }
    Ok(())
}
```

Add `conversation_context: Option<String>` to `QueuedTask` struct.

Update `on_session_complete` to pass it through when dequeuing:
```rust
self.manager.start_session(queued.task, &queued.stage, queued.conversation_context).await?;
```

**Step 4: Add `comment_repo` to TaskApiState**

In `api/mod.rs`:
```rust
#[derive(Clone)]
pub struct TaskApiState {
    pub repo: TaskRepository,
    pub comment_repo: CommentRepository,  // ADD
    pub queue: Option<Arc<SessionQueue>>,
}

impl From<AppState> for TaskApiState {
    fn from(state: AppState) -> Self {
        TaskApiState {
            repo: state.tasks,
            comment_repo: state.comments,  // ADD
            queue: state.queue,
        }
    }
}
```

**Step 5: Fix existing `start_session` call in tasks.rs**

Find `queue.enqueue(task, stage)` call and add `None`:
```rust
match queue.enqueue(task, stage, None).await {
```

**Step 6: Add `POST /api/tasks/:id/sessions/continue` route**

In `tasks.rs`, add route and handler:

```rust
pub fn task_routes() -> Router<TaskApiState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).patch(update_task).delete(delete_task))
        .route("/:id/move", post(move_task))
        .route("/:id/sessions", post(start_session))
        .route("/:id/sessions/continue", post(continue_session))  // ADD
}

#[instrument(skip(state))]
async fn continue_session(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "API: Continuing Claude session with comment history");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Queue not available" }))).into_response(),
    };

    // Build conversation context from comment thread
    let comments = state.comment_repo.list_for_task(&id).await.unwrap_or_default();
    let conversation_context = if comments.is_empty() {
        None
    } else {
        let history = comments.iter().flat_map(|c| {
            let prefix = if c.author == "claude" { "[Claude]" } else { "[You]" };
            let mut lines = vec![format!("{}: {}", prefix, c.content)];
            for reply in &c.replies {
                let rprefix = if reply.author == "claude" { "[Claude]" } else { "[You]" };
                lines.push(format!("  {}: {}", rprefix, reply.content));
            }
            lines
        }).collect::<Vec<_>>().join("\n");
        Some(history)
    };

    let stage = task.stage.clone();
    match queue.enqueue(task, stage, conversation_context).await {
        Ok(()) => Json(serde_json::json!({ "status": "queued" })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}
```

**Step 7: Verify compile**
```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "^error"
```

**Step 8: Commit**
```bash
git add backend/src/claude/prompts.rs backend/src/claude/manager.rs backend/src/claude/queue.rs backend/src/api/tasks.rs backend/src/api/mod.rs
git commit -m "feat: add continue_session endpoint with comment history as conversation context"
```

---

### Task 6: WebSocket per-client session filtering

**Why:** Currently all session output is broadcast to every connected client. When a user is viewing task A, they shouldn't receive task B's output. Clients send `SubscribeSession { session_id }` — the backend should honor this.

**Files:**
- Modify: `backend/src/ws/handler.rs`

**Step 1: Rewrite handler to filter by subscribed session_id**

Replace the contents of `handle_socket`:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

async fn handle_socket(socket: WebSocket, manager: Arc<ClaudeManager>) {
    let (mut sender, mut receiver) = socket.split();
    let mut output_rx = manager.subscribe();

    info!("WebSocket client connected");

    // Shared, atomically updated session subscription
    let subscribed_session: Arc<tokio::sync::RwLock<Option<String>>> =
        Arc::new(tokio::sync::RwLock::new(None));
    let subscribed_session_send = subscribed_session.clone();

    let mut send_task = tokio::spawn(async move {
        while let Ok(output) = output_rx.recv().await {
            // Filter: only forward if client subscribed to this session (or subscribed to all)
            let sub = subscribed_session_send.read().await;
            let should_send = match sub.as_deref() {
                Some(id) => id == output.session_id,
                None => true, // not yet subscribed → broadcast all (backwards compat)
            };
            drop(sub);

            if !should_send {
                continue;
            }

            let msg = ServerMessage::session_output(output.session_id, output.line, output.is_error);
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => { error!("Failed to serialize: {}", e); continue; }
            };
            if sender.send(Message::Text(json)).await.is_err() {
                debug!("Client disconnected (send)");
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = &msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::SubscribeSession { session_id }) => {
                        info!(session_id = %session_id, "Client subscribed to session");
                        let mut sub = subscribed_session.write().await;
                        *sub = Some(session_id.clone());
                    }
                    Ok(ClientMessage::Ping) => {
                        debug!("Ping received");
                    }
                    Ok(_) => {}
                    Err(e) => warn!("Failed to parse client message: {}", e),
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket client disconnected");
}
```

**Step 2: Verify compile**
```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "^error"
```

**Step 3: Commit**
```bash
git add backend/src/ws/handler.rs
git commit -m "feat: WebSocket per-client session filtering via SubscribeSession message"
```

---

### Task 7: Frontend — `useSession` hook and session status polling

**Why:** `task.session_id` is set but the frontend has no way to know the session's current status (running/completed/failed). We need a hook to poll `GET /api/sessions/{id}`.

**Files:**
- Create: `frontend/src/hooks/use-sessions.ts`
- Modify: `frontend/src/types/session.ts` (or create)

**Step 1: Check/create the Session type**

Check `frontend/src/types/session.ts`. If it doesn't match the backend, update it:

```typescript
// frontend/src/types/session.ts
export type SessionStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface Session {
  id: string;
  task_id: string;
  status: SessionStatus;
  started_at: string;
  ended_at: string | null;
  error_message: string | null;
}
```

**Step 2: Create `use-sessions.ts`**

```typescript
// frontend/src/hooks/use-sessions.ts
'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient, ApiError } from '@/lib/api-client';
import type { Session } from '@/types/session';

export function useSession(sessionId: string | null | undefined) {
  return useQuery({
    queryKey: ['sessions', sessionId],
    queryFn: async () => {
      try {
        return await apiClient<Session>(`/api/sessions/${sessionId}`);
      } catch (e) {
        if (e instanceof ApiError && e.status === 404) return null;
        throw e;
      }
    },
    enabled: !!sessionId,
    refetchInterval: (query) => {
      // Poll while running or pending; stop when completed/failed
      const status = query.state.data?.status;
      if (status === 'running' || status === 'pending') return 3000;
      return false;
    },
  });
}

export function useStopSession(taskId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (sessionId: string) =>
      apiClient<void>(`/api/sessions/${sessionId}/stop`, { method: 'POST' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
      queryClient.invalidateQueries({ queryKey: ['sessions'] });
    },
  });
}
```

**Step 3: Commit**
```bash
git add frontend/src/hooks/use-sessions.ts frontend/src/types/session.ts
git commit -m "feat: add useSession hook with status polling"
```

---

### Task 8: Frontend — `LiveOutputPanel` component

**Why:** Shows Claude's real-time output stream in the task detail while a session is running.

**Files:**
- Create: `frontend/src/components/sessions/live-output-panel.tsx`
- Modify: `frontend/src/contexts/websocket-context.tsx` (verify subscribe API works)

**Step 1: Read the WebSocket context to understand subscribe API**

Check `frontend/src/contexts/websocket-context.tsx` — the `subscribe(eventType, callback)` returns an unsubscribe function. Also check the `send` function signature.

**Step 2: Create `LiveOutputPanel`**

```typescript
// frontend/src/components/sessions/live-output-panel.tsx
'use client';

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import type { SessionStatus } from '@/types/session';

interface Props {
  sessionId: string;
  status: SessionStatus | null | undefined;
}

export function LiveOutputPanel({ sessionId, status }: Props) {
  const [lines, setLines] = useState<{ text: string; isError: boolean }[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);
  const { subscribe, send, isConnected } = useWebSocket();

  // Subscribe to this session's output
  useEffect(() => {
    if (!sessionId || !isConnected) return;

    // Tell the server we want this session's output
    send({ type: 'subscribe_session', session_id: sessionId });

    const unsub = subscribe('session_output', (msg: any) => {
      if (msg.session_id !== sessionId) return;
      setLines((prev) => [...prev.slice(-500), { text: msg.output, isError: msg.is_error }]);
    });

    return unsub;
  }, [sessionId, isConnected]);

  // Reset lines when session changes
  useEffect(() => {
    setLines([]);
  }, [sessionId]);

  // Auto-scroll to bottom
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [lines]);

  if (!sessionId) return null;

  const isRunning = status === 'running' || status === 'pending';

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-muted/40 border-b border-border">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Session Output
        </span>
        {isRunning && (
          <span className="flex items-center gap-1.5 text-xs text-emerald-500">
            <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
            Live
          </span>
        )}
        {!isRunning && lines.length > 0 && (
          <span className="text-xs text-muted-foreground">Completed</span>
        )}
      </div>
      <div className="max-h-64 overflow-y-auto bg-black/90 p-3 font-mono text-xs">
        {lines.length === 0 ? (
          <p className="text-muted-foreground italic">
            {isRunning ? 'Waiting for output...' : 'No output captured.'}
          </p>
        ) : (
          lines.map((line, i) => (
            <div
              key={i}
              className={line.isError ? 'text-red-400' : 'text-green-300/90'}
            >
              {line.text}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
```

**Step 3: Check WebSocket context send signature**

The `send` function in `websocket-context.tsx` likely takes an object. Verify and adjust the `send({ type: 'subscribe_session', session_id: sessionId })` call to match the actual API (it may need `JSON.stringify`).

**Step 4: Commit**
```bash
git add frontend/src/components/sessions/live-output-panel.tsx
git commit -m "feat: add LiveOutputPanel component for real-time session output"
```

---

### Task 9: Frontend — Simplified `SessionControls` with Continue Session

**Why:** Remove non-functional Pause/Resume. Add "Continue Session" button when there's prior Claude comment history. Show correct status from `useSession`.

**Files:**
- Modify: `frontend/src/components/sessions/session-controls.tsx`

**Step 1: Rewrite SessionControls**

```typescript
'use client';

import { Play, Square, RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { apiClient } from '@/lib/api-client';
import { useQueryClient } from '@tanstack/react-query';
import type { SessionStatus } from '@/types/session';

interface SessionControlsProps {
  taskId: string;
  sessionId?: string | null;
  status?: SessionStatus | null;
  hasClaudeComments?: boolean;
}

export function SessionControls({
  taskId,
  sessionId,
  status,
  hasClaudeComments = false,
}: SessionControlsProps) {
  const queryClient = useQueryClient();

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
    queryClient.invalidateQueries({ queryKey: ['sessions'] });
    queryClient.invalidateQueries({ queryKey: ['comments', taskId] });
  };

  const startSession = async () => {
    await apiClient(`/api/tasks/${taskId}/sessions`, { method: 'POST' });
    invalidate();
  };

  const continueSession = async () => {
    await apiClient(`/api/tasks/${taskId}/sessions/continue`, { method: 'POST' });
    invalidate();
  };

  const stopSession = async () => {
    if (!sessionId) return;
    await apiClient(`/api/sessions/${sessionId}/stop`, { method: 'POST' });
    invalidate();
  };

  if (status === 'pending') {
    return (
      <Button disabled size="sm">
        <Play className="mr-2 h-4 w-4" />
        Starting...
      </Button>
    );
  }

  if (status === 'running') {
    return (
      <Button onClick={stopSession} variant="destructive" size="sm">
        <Square className="mr-2 h-4 w-4" />
        Stop Session
      </Button>
    );
  }

  // completed, failed, or no session
  return (
    <div className="flex gap-2">
      <Button onClick={startSession} size="sm">
        <Play className="mr-2 h-4 w-4" />
        Start Session
      </Button>
      {hasClaudeComments && (
        <Button onClick={continueSession} variant="outline" size="sm">
          <RotateCcw className="mr-2 h-4 w-4" />
          Continue Session
        </Button>
      )}
    </div>
  );
}
```

**Step 2: Commit**
```bash
git add frontend/src/components/sessions/session-controls.tsx
git commit -m "feat: simplify SessionControls — remove pause/resume, add Continue Session"
```

---

### Task 10: Frontend — Wire task detail with real session status + LiveOutputPanel

**Why:** Task detail currently passes `status={task.session_id ? 'running' : undefined}` which is wrong. It needs to fetch the actual session status, show LiveOutputPanel, and pass `hasClaudeComments` to SessionControls.

**Files:**
- Modify: `frontend/src/components/tasks/task-detail.tsx`

**Step 1: Update task-detail.tsx**

Import additions:
```typescript
import { useSession } from '@/hooks/use-sessions';
import { LiveOutputPanel } from '@/components/sessions/live-output-panel';
```

Inside `TaskDetail`, after the existing hooks, add:
```typescript
const { data: session } = useSession(task.session_id);
const sessionStatus = session?.status ?? null;
const hasClaudeComments = comments.some((c) => c.author === 'claude');
```

Replace the `<TaskSection title="Session">` block:
```typescript
<TaskSection title="Session">
  <div className="space-y-3">
    <SessionControls
      taskId={task.id}
      sessionId={task.session_id}
      status={sessionStatus}
      hasClaudeComments={hasClaudeComments}
    />
    {task.session_id && (
      <>
        <p className="text-xs text-muted-foreground font-mono">
          Session: {task.session_id}
        </p>
        <LiveOutputPanel sessionId={task.session_id} status={sessionStatus} />
      </>
    )}
  </div>
</TaskSection>
```

**Step 2: Commit**
```bash
git add frontend/src/components/tasks/task-detail.tsx
git commit -m "feat: wire task detail with real session status and LiveOutputPanel"
```

---

### Task 11: Frontend — New Task button on Kanban board

**Files:**
- Modify: `frontend/src/app/kanban/page.tsx`

**Step 1: Add CreateTaskDialog to kanban page**

`CreateTaskDialog` is in `@/components/tasks/create-task-dialog`. Add it:

```typescript
'use client';

import { useState, Suspense } from 'react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { CreateTaskDialog } from '@/components/tasks/create-task-dialog';
import { useTasks } from '@/hooks/use-tasks';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';

function KanbanContent() {
  const { data: tasks = [], isLoading, error } = useTasks();
  const [createOpen, setCreateOpen] = useState(false);

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-destructive">Error loading tasks: {error.message}</p>
      </div>
    );
  }

  return (
    <>
      <KanbanBoard tasks={tasks} isLoading={isLoading} />
      <CreateTaskDialog open={createOpen} onOpenChange={setCreateOpen} />
    </>
  );
}

export default function KanbanPage() {
  const [createOpen, setCreateOpen] = useState(false);

  return (
    <div className="h-screen bg-background p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Kanban Board</h1>
        <Button onClick={() => setCreateOpen(true)} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          New Task
        </Button>
      </div>
      <Suspense fallback={<div>Loading...</div>}>
        <KanbanContent />
      </Suspense>
      <CreateTaskDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}
```

Note: `CreateTaskDialog` needs `open`/`onOpenChange` props — check its current interface and adjust if needed.

**Step 2: Commit**
```bash
git add frontend/src/app/kanban/page.tsx
git commit -m "feat: add New Task button to Kanban board"
```

---

### Task 12: Frontend — Delete button on comments

**Files:**
- Modify: `frontend/src/components/tasks/comment-thread.tsx`

**Step 1: Add delete button to SingleComment**

Update `CommentThread` to accept and use `useDeleteComment`:

```typescript
import { useDeleteComment } from '@/hooks/use-comments';
import { Trash2 } from 'lucide-react';

function SingleComment({
  comment,
  isReply = false,
  taskId,
}: {
  comment: Comment;
  isReply?: boolean;
  taskId: string;
}) {
  const { mutate: deleteComment, isPending } = useDeleteComment(taskId);
  const authorLabel = comment.author === 'claude' ? 'Claude' : 'You';
  const timeAgo = formatDistanceToNow(new Date(comment.created_at), { addSuffix: true });

  return (
    <div className={`${isReply ? 'ml-8 mt-2' : 'mb-4'} group`}>
      <div className="flex items-center gap-2 mb-1">
        <span className={`text-sm font-medium ${comment.author === 'claude' ? 'text-purple-600 dark:text-purple-400' : 'text-foreground'}`}>
          {authorLabel}
        </span>
        <span className="text-xs text-muted-foreground">{timeAgo}</span>
        {comment.author !== 'claude' && (
          <button
            onClick={() => deleteComment(comment.id)}
            disabled={isPending}
            className="ml-auto opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"
            title="Delete comment"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
      <p className="text-sm whitespace-pre-wrap">{comment.content}</p>
    </div>
  );
}
```

Pass `taskId` down through `CommentThread`:
```typescript
{comments.map((comment) => (
  <div key={comment.id} ...>
    <SingleComment comment={comment} taskId={taskId} />
    {comment.replies.map((reply) => (
      <SingleComment key={reply.id} comment={reply} isReply taskId={taskId} />
    ))}
    ...
  </div>
))}
```

**Step 2: Commit**
```bash
git add frontend/src/components/tasks/comment-thread.tsx
git commit -m "feat: add delete button to user comments in CommentThread"
```

---

### Task 13: Rebuild, restart, and verify end-to-end

**Step 1: Stop apps**
```bash
cd /home/utility/Projects/ai-kanban && bash stop.sh
```

**Step 2: Build backend**
```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | tail -5
```
Expected: `Finished \`dev\` profile`

**Step 3: Start apps**
```bash
cd /home/utility/Projects/ai-kanban && bash start.sh
```

**Step 4: Manual smoke test checklist**
- [ ] Kanban board shows "+ New Task" button, creates a task
- [ ] Task detail shows Instructions/Context editable fields
- [ ] Clicking "Start Session" kicks off Claude, `task.session_id` gets set
- [ ] LiveOutputPanel appears and shows live output
- [ ] Session completes → status shows "completed" → Claude comment appears in thread
- [ ] "Continue Session" button appears; clicking it starts a new session with conversation
- [ ] Moving task to Done stops running session
- [ ] Deleting a task with a running session kills the process
- [ ] Comment delete button appears on hover for user comments

**Step 5: Final commit**
```bash
git add -A
git commit -m "chore: final integration — interactive sessions, session lifecycle, kanban improvements"
```
