# Mobile, LiteLLM Summarization & Image Attachments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Three independent feature areas: (A) make the task list and sidebar metrics usable on mobile, (B) improve LiteLLM session summaries with structured output and performance metrics, (C) add image attachment support for tasks that gets passed to Claude sessions.

**Architecture:**
- Group A is frontend-only; touches `task-list.tsx`, `sidebar.tsx`, `page.tsx`, `task-detail.tsx`.
- Group B is backend-only; `litellm.rs` gains latency tracking returned in `CompletionResult`, `context_manager.rs` gets a richer prompt and comment format.
- Group C is full-stack: new DB table → Rust repo → Axum API → Next.js hooks/components → Claude CLI args. Images stored in `.claude/attachments/` inside each task's project directory so Claude can read them natively.

**Tech Stack:** Next.js 16 / Tailwind CSS, Rust / Axum / sqlx / SQLite, Claude Code CLI, LiteLLM OpenAI-compatible API

---

## Group A — Mobile Responsiveness

### Task 1: Fix task-list mobile layout

**Files:**
- Modify: `frontend/src/components/tasks/task-list.tsx`

The toolbar row (view mode toggles + sort dropdown) overflows on narrow screens. The grid view uses fixed columns. Compact/list rows need touch-friendly tap targets.

- [ ] **Step 1: Read the current task-list component**

Read `frontend/src/components/tasks/task-list.tsx` in full (it's ~300 lines). Understand the toolbar structure and the three view modes (grid, list, compact).

- [ ] **Step 2: Fix the toolbar row**

The toolbar wraps view-mode buttons, sort dropdown, and "New task" button. On mobile the row overflows. Change the outer toolbar div to use `flex-wrap` and hide the view-mode toggle group on mobile (it's unnecessary — default to list view on mobile):

```tsx
// In the toolbar div, add gap and flex-wrap:
<div className="flex flex-wrap items-center gap-2 mb-4">
  {/* Hide grid/compact toggles on mobile — always use list on small screens */}
  <div className="hidden sm:flex items-center gap-1 ...">
    {/* existing view mode buttons */}
  </div>
  {/* sort dropdown and new task button stay visible */}
</div>
```

Also force `viewMode` to `'list'` on mobile by reading window width or using a Tailwind class trick on the container. The cleanest way: add `useEffect` that sets view mode to `'list'` when `window.innerWidth < 640`, and re-check on resize. Or simply hide grid/compact buttons and let the user keep whichever mode they set last — hiding the buttons is enough.

- [ ] **Step 3: Fix grid view on mobile**

The grid currently uses `grid-cols-1 sm:grid-cols-2 lg:grid-cols-3`. Verify this is correct — one column on mobile, two on small screens. If it uses a fixed number, change to `grid-cols-1 sm:grid-cols-2 lg:grid-cols-3`.

- [ ] **Step 4: Fix list/compact row tap targets**

`ListRow` and compact rows link via `<Link>`. Ensure padding is at least `py-3 px-4` (currently correct per code). Add `min-h-[48px]` to ensure 48px touch target on the row div.

- [ ] **Step 5: Verify filter/search bar if present**

Check if there is a stage filter bar or search input at the top. If so, ensure it uses `w-full` on mobile and doesn't overflow.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/tasks/task-list.tsx
git commit -m "fix(mobile): responsive task list layout"
```

---

### Task 2: Add mobile metrics strip

**Files:**
- Modify: `frontend/src/components/layout/sidebar.tsx`
- Modify: `frontend/src/app/page.tsx`

On mobile the sidebar collapses to a bottom nav bar, hiding the 6 usage metrics. Add a horizontally-scrollable metrics strip that appears at the top of the main content on mobile only.

- [ ] **Step 1: Extract the metrics data hook into a shared hook**

In `frontend/src/components/layout/sidebar.tsx`, the `SidebarMetrics` component fetches from `/api/analytics/overview`. Extract the hook call and formatting logic into a new exported function `useSidebarMetrics()` at the bottom of the same file, or create `frontend/src/hooks/use-sidebar-metrics.ts`:

```ts
// frontend/src/hooks/use-sidebar-metrics.ts
'use client';
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type { AnalyticsOverview } from '@/types/analytics';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function useSidebarMetrics() {
  const { data } = useQuery<AnalyticsOverview>({
    queryKey: ['analytics', 'overview'],
    queryFn: () => apiClient<AnalyticsOverview>('/api/analytics/overview'),
    staleTime: 60_000,
    refetchInterval: 60_000,
  });
  if (!data) return null;

  const totalTokens = data.total_input_tokens + data.total_output_tokens
    + (data.total_cache_creation_tokens ?? 0) + (data.total_cache_read_tokens ?? 0);
  const avgCostPerSession = data.total_sessions > 0
    ? `$${(data.estimated_cost_usd / data.total_sessions).toFixed(2)}`
    : '—';
  const cacheTotal = (data.total_cache_creation_tokens ?? 0) + (data.total_cache_read_tokens ?? 0);
  const cacheHit = totalTokens > 0
    ? `${Math.round((cacheTotal / totalTokens) * 100)}%`
    : '—';

  return [
    { label: 'Cost', value: `$${data.estimated_cost_usd.toFixed(2)}` },
    { label: 'Sessions', value: String(data.total_sessions) },
    { label: 'Tokens', value: fmt(totalTokens) },
    { label: 'Tasks w/ AI', value: String(data.total_tasks_with_sessions) },
    { label: 'Avg/Session', value: avgCostPerSession },
    { label: 'Cache Hit', value: cacheHit },
  ];
}
```

- [ ] **Step 2: Update SidebarMetrics in sidebar.tsx to use the shared hook**

Replace the inline fetch in `SidebarMetrics` with `useSidebarMetrics()`. The sidebar component stays visually identical.

- [ ] **Step 3: Add MobileMetricsStrip component to page.tsx**

In `frontend/src/app/page.tsx`, add a `MobileMetricsStrip` component above `StatsStrip` in the `HomeContent` layout. It is only visible on mobile (`md:hidden`):

```tsx
// In frontend/src/app/page.tsx
import { useSidebarMetrics } from '@/hooks/use-sidebar-metrics';

function MobileMetricsStrip() {
  const metrics = useSidebarMetrics();
  if (!metrics) return null;
  return (
    <div className="md:hidden -mx-4 px-4 mb-4 overflow-x-auto">
      <div className="flex gap-3 pb-1 min-w-max">
        {metrics.map((m) => (
          <div key={m.label} className="flex flex-col items-center bg-muted/50 rounded-lg px-3 py-1.5 min-w-[64px]">
            <span className="text-[10px] text-muted-foreground whitespace-nowrap">{m.label}</span>
            <span className="text-xs font-semibold tabular-nums">{m.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
```

Add `<MobileMetricsStrip />` inside `HomeContent`, right before `<StatsStrip />`.

- [ ] **Step 4: Also add to kanban page**

Read `frontend/src/app/kanban/page.tsx`. Add `<MobileMetricsStrip />` in the same position in the kanban page's mobile layout section.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/hooks/use-sidebar-metrics.ts \
        frontend/src/components/layout/sidebar.tsx \
        frontend/src/app/page.tsx \
        frontend/src/app/kanban/page.tsx
git commit -m "feat(mobile): add scrollable metrics strip on mobile"
```

---

### Task 3: Fix task-detail mobile layout

**Files:**
- Modify: `frontend/src/components/tasks/task-detail.tsx`

Task detail opens as a sheet/modal. On mobile it should be full-width and full-height. Check the sheet/dialog component used.

- [ ] **Step 1: Read task-detail.tsx opening structure**

Read `frontend/src/components/tasks/task-detail.tsx` lines 1-80. Identify the sheet/dialog wrapper and what `className` or size props it accepts.

- [ ] **Step 2: Ensure full-screen on mobile**

If the sheet uses shadcn's `Sheet` component with `side="right"`, add `className="w-full sm:max-w-2xl"` to the `SheetContent`. If it's a dialog, add `className="max-h-[90vh] overflow-y-auto sm:max-w-2xl"`.

- [ ] **Step 3: Fix inner layout overflow**

Look for any `flex gap-4` or `grid grid-cols-2` layouts inside the task detail that would overflow on narrow screens. Change to `flex-col sm:flex-row` or `grid-cols-1 sm:grid-cols-2`.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/tasks/task-detail.tsx
git commit -m "fix(mobile): task detail sheet full-screen on mobile"
```

---

## Group B — LiteLLM Summarization Improvements

### Task 4: Add latency and throughput to LiteLLM client

**Files:**
- Modify: `backend/src/ai/litellm.rs`

Add wall-clock latency (ms from request send → response received) and tokens/sec to `CompletionResult`. The caller uses these to build the comment footer.

- [ ] **Step 1: Add timing fields to CompletionResult**

```rust
// In backend/src/ai/litellm.rs
#[derive(Debug)]
pub struct CompletionResult {
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub latency_ms: u64,       // wall-clock ms for the HTTP round-trip
    pub tokens_per_sec: f64,   // output_tokens / (latency_ms / 1000)
}
```

- [ ] **Step 2: Record timing around the HTTP call in `complete()`**

```rust
// In the complete() function, wrap the send/await:
let t_start = std::time::Instant::now();
let response = self.client
    .post(&url)
    // ... existing headers ...
    .send()
    .await?;
// ... existing status check and json parse ...
let latency_ms = t_start.elapsed().as_millis() as u64;
let tokens_per_sec = if latency_ms > 0 {
    output_tokens as f64 / (latency_ms as f64 / 1000.0)
} else {
    0.0
};

Ok(CompletionResult { content, input_tokens, output_tokens, latency_ms, tokens_per_sec })
```

- [ ] **Step 3: Build and check for compile errors**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | head -40
```

Fix any compile errors (there will be unused field warnings from context_manager.rs — those get fixed in Task 5).

- [ ] **Step 4: Commit**

```bash
git add backend/src/ai/litellm.rs
git commit -m "feat(litellm): add latency_ms and tokens_per_sec to CompletionResult"
```

---

### Task 5: Improve summarization prompt and comment format

**Files:**
- Modify: `backend/src/ai/context_manager.rs`

Richer context in prompt, structured markdown output requested, higher word budget, performance metrics in comment footer.

- [ ] **Step 1: Update `summarize_session` signature to accept more context**

Add `task_stage: &str`, `session_duration_secs: Option<u64>`, `input_tokens: i64`, `output_tokens: i64` parameters. These let us include richer context in the prompt and comment.

Find where `summarize_session` is called in `backend/src/claude/manager.rs` and update the call site to pass the extra args. The session's `duration_secs` and token counts are available in the session record.

- [ ] **Step 2: Increase activity lines from 300 → 500**

```rust
let activity_lines: Vec<&str> = display_lines.iter()
    .take(500)   // was 300
    .map(|s| s.as_str())
    .collect();
```

Also increase final output preview from 600 → 1200 chars.

- [ ] **Step 3: Update the system prompt**

```rust
let system_prompt = "You are a technical project assistant that writes structured session summaries for an AI-assisted development tool. \
Write in clear, specific language. Use exact file names and function names when they appear in the activity log. \
Never use vague phrases like 'various changes' or 'several files'. \
Format output as markdown with the three sections below — fill each one thoroughly. Aim for 200-300 words total.";
```

- [ ] **Step 4: Update the user prompt to request structured output**

```rust
let duration_str = match session_duration_secs {
    Some(d) if d >= 60 => format!("{}m {}s", d / 60, d % 60),
    Some(d) => format!("{}s", d),
    None => "unknown".to_string(),
};

let user_content = format!(
    "Task: {task_title} (stage: {task_stage})\n\
     Session stats: {duration_str} · {in_tok} input / {out_tok} output tokens\n\
     Activity ({total} events, showing up to 500):\n{activity}{result_section}\n\n\
     Write a summary using exactly these three sections:\n\
     ## What Changed\n\
     (bullet list of concrete changes — be specific about file names and what was done)\n\n\
     ## Files Modified\n\
     (bullet list: `filename` — one-line description of change)\n\n\
     ## Notes\n\
     (one to three sentences on decisions made, blockers hit, or next steps — omit if nothing notable)",
    task_title = task_title,
    task_stage = task_stage,
    duration_str = duration_str,
    in_tok = input_tokens,
    out_tok = output_tokens,
    total = display_lines.len(),
    activity = activity,
    result_section = result_section,
);
```

- [ ] **Step 5: Update the comment format to include perf metrics footer**

```rust
let perf_line = format!(
    "⚡ *LiteLLM · {}ms · {:.0} tok/s · {} in / {} out*",
    result.latency_ms,
    result.tokens_per_sec,
    result.input_tokens,
    result.output_tokens,
);

let comment_content = format!(
    "**Session Summary**\n\n{}{}\n\n---\n{}",
    result.content.trim(),
    files_section,
    perf_line,
);
```

- [ ] **Step 6: Build and fix compile errors**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | head -60
```

- [ ] **Step 7: Commit**

```bash
git add backend/src/ai/context_manager.rs backend/src/claude/manager.rs
git commit -m "feat(litellm): structured summary output, 500-line context, perf metrics footer"
```

---

## Group C — Image Attachments

### Task 6: DB migration and Rust model

**Files:**
- Create: `backend/migrations/015_task_attachments.sql`
- Create: `backend/src/models/attachment.rs`
- Modify: `backend/src/models/mod.rs`

- [ ] **Step 1: Write the migration**

```sql
-- backend/migrations/015_task_attachments.sql
CREATE TABLE IF NOT EXISTS task_attachments (
    id           TEXT PRIMARY KEY,
    task_id      TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    mime_type    TEXT NOT NULL DEFAULT 'application/octet-stream',
    size_bytes   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_task_attachments_task_id ON task_attachments(task_id);
```

- [ ] **Step 2: Write the Rust model**

```rust
// backend/src/models/attachment.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskAttachment {
    pub id: String,
    pub task_id: String,
    pub filename: String,
    pub storage_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 3: Export from models/mod.rs**

Add `pub mod attachment;` and `pub use attachment::TaskAttachment;` to `backend/src/models/mod.rs`.

- [ ] **Step 4: Build to verify model compiles**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "error|warning: unused" | head -20
```

- [ ] **Step 5: Commit**

```bash
git add backend/migrations/015_task_attachments.sql \
        backend/src/models/attachment.rs \
        backend/src/models/mod.rs
git commit -m "feat(attachments): DB migration and TaskAttachment model"
```

---

### Task 7: Attachment repository

**Files:**
- Create: `backend/src/db/attachments.rs`
- Modify: `backend/src/db/mod.rs`

- [ ] **Step 1: Write the repository**

```rust
// backend/src/db/attachments.rs
use crate::models::TaskAttachment;
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AttachmentRepository {
    pool: SqlitePool,
}

impl AttachmentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, attachment: &TaskAttachment) -> Result<TaskAttachment> {
        let row = sqlx::query_as::<_, TaskAttachment>(
            "INSERT INTO task_attachments (id, task_id, filename, storage_path, mime_type, size_bytes, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&attachment.id)
        .bind(&attachment.task_id)
        .bind(&attachment.filename)
        .bind(&attachment.storage_path)
        .bind(&attachment.mime_type)
        .bind(attachment.size_bytes)
        .bind(&attachment.created_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_for_task(&self, task_id: &str) -> Result<Vec<TaskAttachment>> {
        let rows = sqlx::query_as::<_, TaskAttachment>(
            "SELECT * FROM task_attachments WHERE task_id = ? ORDER BY created_at ASC"
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get(&self, id: &str) -> Result<Option<TaskAttachment>> {
        let row = sqlx::query_as::<_, TaskAttachment>(
            "SELECT * FROM task_attachments WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM task_attachments WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

- [ ] **Step 2: Export from db/mod.rs**

Add `pub mod attachments;` and `pub use attachments::AttachmentRepository;` to `backend/src/db/mod.rs`.

- [ ] **Step 3: Build**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | head -30
```

- [ ] **Step 4: Commit**

```bash
git add backend/src/db/attachments.rs backend/src/db/mod.rs
git commit -m "feat(attachments): AttachmentRepository with CRUD operations"
```

---

### Task 8: Attachment API handlers and routes

**Files:**
- Create: `backend/src/api/attachments.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/api/routes.rs`
- Modify: `backend/src/main.rs` (add AttachmentRepository to AppState)

Storage directory: use `ATTACHMENTS_DIR` env var, default `~/.ai-kanban/attachments`. Each task gets a subdirectory: `<ATTACHMENTS_DIR>/<task_id>/`.

- [ ] **Step 1: Add AttachmentRepository to AppState**

Read `backend/src/api/mod.rs` and `backend/src/main.rs`. Add `pub attachments: AttachmentRepository` to `AppState`, initialize it in `AppState::new()`, and thread it through to the attachment API state.

- [ ] **Step 2: Create AttachmentApiState**

```rust
// In backend/src/api/mod.rs, add:
#[derive(Clone)]
pub struct AttachmentApiState {
    pub repo: AttachmentRepository,
    pub task_repo: TaskRepository,
    pub attachments_dir: String,
}
```

- [ ] **Step 3: Write API handlers**

```rust
// backend/src/api/attachments.rs
use crate::api::AttachmentApiState;
use crate::models::TaskAttachment;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use tokio::fs;
use uuid::Uuid;

// GET /api/tasks/:task_id/attachments
pub async fn list_attachments(
    State(state): State<AttachmentApiState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<TaskAttachment>>, StatusCode> {
    state.repo.list_for_task(&task_id).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// POST /api/tasks/:task_id/attachments  (multipart/form-data, field name: "file")
pub async fn upload_attachment(
    State(state): State<AttachmentApiState>,
    Path(task_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<TaskAttachment>, StatusCode> {
    // Verify task exists
    state.task_repo.get(&task_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let filename = field.file_name()
            .unwrap_or("upload")
            .to_string();
        let mime_type = field.content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

        // Write to disk: <attachments_dir>/<task_id>/<uuid>-<filename>
        let dir = format!("{}/{}", state.attachments_dir, task_id);
        fs::create_dir_all(&dir).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let id = Uuid::new_v4().to_string();
        let safe_name = filename.replace(['/', '\\', '..'], "_");
        let storage_path = format!("{}/{}-{}", dir, id, safe_name);
        fs::write(&storage_path, &data).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let attachment = TaskAttachment {
            id,
            task_id,
            filename,
            storage_path,
            mime_type,
            size_bytes: data.len() as i64,
            created_at: Utc::now(),
        };

        return state.repo.create(&attachment).await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }
    Err(StatusCode::BAD_REQUEST)
}

// DELETE /api/tasks/:task_id/attachments/:attachment_id
pub async fn delete_attachment(
    State(state): State<AttachmentApiState>,
    Path((task_id, attachment_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let attachment = state.repo.get(&attachment_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if attachment.task_id != task_id {
        return Err(StatusCode::NOT_FOUND);
    }

    // Delete file from disk (best-effort)
    let _ = fs::remove_file(&attachment.storage_path).await;
    state.repo.delete(&attachment_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// GET /api/tasks/:task_id/attachments/:attachment_id/file  — serve file
pub async fn serve_attachment(
    State(state): State<AttachmentApiState>,
    Path((task_id, attachment_id)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    let attachment = state.repo.get(&attachment_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if attachment.task_id != task_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let data = fs::read(&attachment.storage_path).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, &attachment.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", attachment.filename),
        )
        .body(Body::from(data))
        .unwrap())
}

pub fn attachment_routes() -> axum::Router<AttachmentApiState> {
    use axum::routing::{delete, get, post};
    axum::Router::new()
        .route("/", get(list_attachments).post(upload_attachment))
        .route("/:attachment_id", delete(delete_attachment))
        .route("/:attachment_id/file", get(serve_attachment))
}
```

- [ ] **Step 4: Register routes in routes.rs**

In `backend/src/api/routes.rs`, add the attachment routes nested under `/api/tasks/:task_id/attachments`:

```rust
// Add to imports:
use crate::api::attachments::{attachment_routes, AttachmentApiState};  // add to api/mod.rs first

// In create_router, add:
let attachment_state = AttachmentApiState {
    repo: state.attachments.clone(),
    task_repo: state.tasks.clone(),
    attachments_dir: std::env::var("ATTACHMENTS_DIR")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.ai-kanban/attachments", home)
        }),
};
// ...
.nest("/api/tasks/:task_id/attachments", attachment_routes().with_state(attachment_state))
```

- [ ] **Step 5: Export from api/mod.rs**

Add `pub mod attachments;` to `backend/src/api/mod.rs`. Move `AttachmentApiState` there if needed.

- [ ] **Step 6: Build and fix errors**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | head -60
```

- [ ] **Step 7: Quick manual test**

```bash
# Start backend
./start.sh

# Test list (should return [])
curl -s http://localhost:3001/api/tasks/<any-task-id>/attachments | jq .

# Test upload
curl -s -X POST http://localhost:3001/api/tasks/<task-id>/attachments \
  -F "file=@/tmp/test.png" | jq .
```

- [ ] **Step 8: Commit**

```bash
git add backend/src/api/attachments.rs \
        backend/src/api/mod.rs \
        backend/src/api/routes.rs \
        backend/src/main.rs
git commit -m "feat(attachments): upload/list/delete/serve API endpoints"
```

---

### Task 9: Frontend types and React Query hooks

**Files:**
- Create: `frontend/src/types/attachment.ts`
- Create: `frontend/src/hooks/use-attachments.ts`

- [ ] **Step 1: Write the TypeScript type**

```ts
// frontend/src/types/attachment.ts
export interface TaskAttachment {
  id: string;
  task_id: string;
  filename: string;
  mime_type: string;
  size_bytes: number;
  created_at: string;
}
```

- [ ] **Step 2: Write the hooks**

```ts
// frontend/src/hooks/use-attachments.ts
'use client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type { TaskAttachment } from '@/types/attachment';

export function useAttachments(taskId: string) {
  return useQuery({
    queryKey: ['attachments', taskId],
    queryFn: () => apiClient<TaskAttachment[]>(`/api/tasks/${taskId}/attachments`),
    enabled: !!taskId,
  });
}

export function useUploadAttachment(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (file: File) => {
      const form = new FormData();
      form.append('file', file);
      const res = await fetch(`/api/tasks/${taskId}/attachments`, {
        method: 'POST',
        body: form,
      });
      if (!res.ok) throw new Error('Upload failed');
      return res.json() as Promise<TaskAttachment>;
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['attachments', taskId] }),
  });
}

export function useDeleteAttachment(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (attachmentId: string) =>
      fetch(`/api/tasks/${taskId}/attachments/${attachmentId}`, { method: 'DELETE' }).then(() => {}),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['attachments', taskId] }),
  });
}

export function attachmentFileUrl(taskId: string, attachmentId: string) {
  return `/api/tasks/${taskId}/attachments/${attachmentId}/file`;
}
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/types/attachment.ts frontend/src/hooks/use-attachments.ts
git commit -m "feat(attachments): TypeScript types and React Query hooks"
```

---

### Task 10: AttachmentZone UI component

**Files:**
- Create: `frontend/src/components/tasks/attachment-zone.tsx`

Drag-and-drop upload zone with thumbnail previews and remove buttons. Compact design to fit in the task detail panel.

- [ ] **Step 1: Write the component**

```tsx
// frontend/src/components/tasks/attachment-zone.tsx
'use client';

import { useRef, useState } from 'react';
import { ImageIcon, Paperclip, Trash2, Upload } from 'lucide-react';
import { useAttachments, useDeleteAttachment, useUploadAttachment, attachmentFileUrl } from '@/hooks/use-attachments';
import type { TaskAttachment } from '@/types/attachment';

function isImage(mime: string) {
  return mime.startsWith('image/');
}

function fmtSize(bytes: number) {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
}

function AttachmentThumb({ attachment, taskId }: { attachment: TaskAttachment; taskId: string }) {
  const del = useDeleteAttachment(taskId);
  const url = attachmentFileUrl(taskId, attachment.id);

  return (
    <div className="group relative flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2 py-1.5 text-xs">
      {isImage(attachment.mime_type) ? (
        <a href={url} target="_blank" rel="noopener noreferrer" className="shrink-0">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={url} alt={attachment.filename} className="h-8 w-8 rounded object-cover" />
        </a>
      ) : (
        <Paperclip className="h-4 w-4 shrink-0 text-muted-foreground" />
      )}
      <div className="flex-1 min-w-0">
        <a href={url} target="_blank" rel="noopener noreferrer"
           className="truncate block text-foreground hover:underline max-w-[120px]">
          {attachment.filename}
        </a>
        <span className="text-muted-foreground">{fmtSize(attachment.size_bytes)}</span>
      </div>
      <button
        onClick={() => del.mutate(attachment.id)}
        disabled={del.isPending}
        className="opacity-0 group-hover:opacity-100 transition-opacity p-0.5 rounded hover:bg-destructive/10 text-muted-foreground hover:text-destructive"
        title="Remove"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

interface Props {
  taskId: string;
}

export function AttachmentZone({ taskId }: Props) {
  const { data: attachments = [] } = useAttachments(taskId);
  const upload = useUploadAttachment(taskId);
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);

  const handleFiles = (files: FileList | null) => {
    if (!files) return;
    Array.from(files).forEach((f) => upload.mutate(f));
  };

  return (
    <div className="space-y-2">
      {/* Existing attachments */}
      {attachments.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {attachments.map((a) => (
            <AttachmentThumb key={a.id} attachment={a} taskId={taskId} />
          ))}
        </div>
      )}

      {/* Upload zone */}
      <div
        className={`relative flex items-center justify-center gap-2 rounded-md border border-dashed px-3 py-2 text-xs text-muted-foreground cursor-pointer transition-colors ${
          dragging ? 'border-primary bg-primary/5' : 'border-border hover:border-primary/50 hover:bg-muted/30'
        }`}
        onClick={() => inputRef.current?.click()}
        onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
        onDragLeave={() => setDragging(false)}
        onDrop={(e) => { e.preventDefault(); setDragging(false); handleFiles(e.dataTransfer.files); }}
      >
        {upload.isPending ? (
          <span className="animate-pulse">Uploading…</span>
        ) : (
          <>
            <Upload className="h-3.5 w-3.5 shrink-0" />
            <span>Attach images or files</span>
          </>
        )}
        <input
          ref={inputRef}
          type="file"
          multiple
          accept="image/*,.pdf,.txt,.md"
          className="hidden"
          onChange={(e) => handleFiles(e.target.files)}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/components/tasks/attachment-zone.tsx
git commit -m "feat(attachments): AttachmentZone upload/preview component"
```

---

### Task 11: Integrate AttachmentZone into task detail

**Files:**
- Modify: `frontend/src/components/tasks/task-detail.tsx`

- [ ] **Step 1: Read task-detail.tsx**

Read the full file. Find the section that contains the description, comments, or action buttons — this is where `AttachmentZone` will appear (just above the session start button or below the description).

- [ ] **Step 2: Add AttachmentZone**

Import `AttachmentZone` and add it in the detail panel. Good placement: in a collapsible section or directly below the task description, labeled "Attachments":

```tsx
import { AttachmentZone } from './attachment-zone';

// In the JSX, add a section:
<div className="space-y-1.5">
  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Attachments</p>
  <AttachmentZone taskId={task.id} />
</div>
```

Place this above the "Start Session" / "Continue Session" button group.

- [ ] **Step 3: Verify visually**

Open a task detail in the browser. The attachment zone should appear with a dashed border upload area. Upload a test image and verify the thumbnail appears.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/tasks/task-detail.tsx
git commit -m "feat(attachments): embed AttachmentZone in task detail panel"
```

---

### Task 12: Pass image attachments to Claude session

**Files:**
- Modify: `backend/src/claude/manager.rs`
- Modify: `backend/src/claude/manager.rs` (write_task_context_file also lists attachments)

When a Claude session starts for a task, fetch that task's attachments and:
1. Copy image files into `.claude/attachments/` inside the project directory (so Claude can see them relative to the project)
2. List them in the task context file so Claude knows to look at them
3. Add `--image <path>` CLI args for image attachments (Claude Code CLI supports this)

- [ ] **Step 1: Add AttachmentRepository to ClaudeManager**

Read `backend/src/claude/manager.rs` top section (struct definition and `new()`). Add `attachment_repo: AttachmentRepository` field and thread it in from wherever `ClaudeManager::new()` is called (likely `main.rs`).

- [ ] **Step 2: Fetch attachments at session start**

In `ClaudeManager::start_session()`, after loading the task and before building the command, fetch the task's attachments:

```rust
let attachments = self.attachment_repo
    .list_for_task(&task.id)
    .await
    .unwrap_or_default();
```

- [ ] **Step 3: Copy images to project's .claude/attachments/ and add --image args**

```rust
// After fetching attachments:
let claude_attachments_dir = format!("{}/.claude/attachments", project_path);
let _ = tokio::fs::create_dir_all(&claude_attachments_dir).await;

for att in &attachments {
    // Copy file into the project's .claude/attachments/ dir so Claude can read it
    let dest = format!("{}/{}", claude_attachments_dir, att.filename);
    if let Err(e) = tokio::fs::copy(&att.storage_path, &dest).await {
        warn!(attachment_id = %att.id, error = %e, "Failed to copy attachment to project dir");
        continue;
    }
    // For image types, pass as --image <absolute path>
    if att.mime_type.starts_with("image/") {
        cmd.arg("--image").arg(&dest);
    }
}
```

- [ ] **Step 4: List attachments in the context file**

In `write_task_context_file()`, add an Attachments section if attachments are non-empty. Since this function currently doesn't have access to attachments, pass them as an additional parameter:

```rust
// Add to the context file template, after the description:
if !attachments.is_empty() {
    context.push_str("\n## Attached Files\n");
    for att in attachments {
        context.push_str(&format!("- `.claude/attachments/{}` ({})\n", att.filename, att.mime_type));
    }
    context.push_str("\nPlease review the attached files as they are relevant to this task.\n");
}
```

- [ ] **Step 5: Build and fix errors**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | head -60
```

- [ ] **Step 6: End-to-end test**

1. Create a task, attach an image via the UI
2. Start a Claude session on that task
3. Check the backend logs — should see "Copied attachment to .claude/attachments/"
4. Verify the context file includes the Attachments section
5. Verify Claude can see/describe the image in its output

- [ ] **Step 7: Commit**

```bash
git add backend/src/claude/manager.rs backend/src/main.rs
git commit -m "feat(attachments): pass images to Claude session via --image args and context file"
```

---

## Final

- [ ] Run `./start.sh` and do a full smoke test: upload image → start session → verify summary has structured sections and perf footer → check mobile on a phone or devtools mobile viewport
- [ ] Commit any remaining fixes
