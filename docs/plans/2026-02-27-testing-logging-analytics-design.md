# Testing, Logging, Analytics & Logs Tab — Design Document

**Date:** 2026-02-27
**Status:** Approved
**Scope:** Frontend test coverage to 80%, frontend logging, Analytics tab, Logs tab

---

## 1. Overview

Four distinct feature areas built on top of the completed Phase 1 MVP:

1. **Frontend Testing** — Vitest + Playwright test suite reaching 80% coverage
2. **Frontend Logging** — Structured logger that ships browser events to the backend `/api/logs` endpoint
3. **Analytics Tab** — Beautiful Recharts dashboard for token usage across all dimensions
4. **Logs Tab** — Filterable, polling log viewer showing frontend + backend logs unified

---

## 2. Backend: Token Analytics Schema & API

### New Database Tables

```sql
-- Granular token events — one row per JSONL line from Claude stdout
CREATE TABLE token_events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id    TEXT NOT NULL REFERENCES sessions(id),
    task_id       TEXT NOT NULL REFERENCES tasks(id),
    event_type    TEXT NOT NULL,  -- 'tool_use', 'message', 'result', 'system'
    tool_name     TEXT,           -- 'Read', 'Write', 'Edit', 'Bash', 'Grep', etc.
    file_ext      TEXT,           -- '.rs', '.ts', '.py' extracted from tool args
    input_tokens  INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    model         TEXT,
    sequence_no   INTEGER,        -- position in the JSONL stream
    timestamp     TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Per-session project metrics captured at session start and updated during run
CREATE TABLE session_metrics (
    session_id     TEXT PRIMARY KEY REFERENCES sessions(id),
    project_files  INTEGER DEFAULT 0,   -- file count at session start
    project_loc    INTEGER DEFAULT 0,   -- lines of code at session start
    lines_written  INTEGER DEFAULT 0,   -- accumulated from Write/Edit tool calls
    lines_deleted  INTEGER DEFAULT 0,
    updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_token_events_session ON token_events(session_id);
CREATE INDEX idx_token_events_task    ON token_events(task_id);
CREATE INDEX idx_token_events_ts      ON token_events(timestamp);
CREATE INDEX idx_token_events_tool    ON token_events(tool_name);
```

### JSONL Parsing Change

`backend/src/claude/manager.rs` currently reads stdout lines but does not parse them. Each line from Claude is valid JSON with a structure like:

```json
{"type": "tool_use", "tool": "Read", "path": "src/main.rs", "input_tokens": 100, "output_tokens": 0}
{"type": "result", "input_tokens": 850, "output_tokens": 320, "model": "claude-sonnet-4-6"}
```

Changes:
- Add `TokenEventRepository` in `backend/src/db/token_events.rs`
- In the stdout reader loop, attempt `serde_json::from_str` on each line
- Extract `event_type`, `tool_name`, `file_ext` (from path/file args), token counts
- Insert a `token_event` row asynchronously via a channel (non-blocking, same pattern as `DbLayer`)
- On session start, snapshot `project_files` and `project_loc` into `session_metrics`
- Accumulate `lines_written`/`lines_deleted` from Write and Edit tool call results

### New Analytics API Endpoints

```
GET /api/analytics/overview
    → { total_input_tokens, total_output_tokens, total_sessions, total_tasks,
        estimated_cost_usd, efficiency_ratio, active_sessions_today }

GET /api/analytics/tokens/daily?days=30
    → [{ date, input_tokens, output_tokens }]

GET /api/analytics/tokens/weekly?weeks=12
    → [{ week_start, input_tokens, output_tokens }]

GET /api/analytics/tokens/monthly?months=6
    → [{ month, input_tokens, output_tokens }]

GET /api/analytics/tokens/by-task
    → [{ task_id, task_title, input_tokens, output_tokens, total }]

GET /api/analytics/tokens/by-session
    → [{ session_id, task_title, input_tokens, output_tokens, started_at }]

GET /api/analytics/tokens/by-tool
    → [{ tool_name, input_tokens, output_tokens, call_count }]

GET /api/analytics/tokens/by-language
    → [{ file_ext, input_tokens, output_tokens, call_count }]

GET /api/analytics/tokens/efficiency
    → [{ task_id, task_title, tokens_per_line_written, tokens_per_loc }]

GET /api/analytics/sessions/:id/timeline
    → [{ sequence_no, event_type, tool_name, input_tokens, output_tokens,
         cumulative_total, timestamp }]
```

All endpoints return JSON. Aggregations are computed via SQL GROUP BY — no separate aggregation pipeline.

---

## 3. Frontend Logging

### Logger Singleton — `src/lib/logger.ts`

```ts
// Usage
logger.debug('component mounted', { component: 'KanbanBoard' })
logger.info('task created', { taskId, title })
logger.warn('websocket reconnecting', { attempt: 3 })
logger.error('api call failed', { endpoint, status, message })

// With context
logger.withContext({ taskId, sessionId }).info('session started')
```

**Behavior:**
- Buffers log entries in memory
- Flushes when buffer reaches 20 entries OR every 10 seconds
- Flushes on `beforeunload` via `navigator.sendBeacon` (best-effort)
- Automatically attaches `source: "frontend"`, timestamp, and user agent
- Flush failures are silently dropped — logging never breaks the UI
- Deduplicates identical consecutive messages within 1 second

### Logger Hook — `src/hooks/use-logger.ts`

Wraps the singleton and auto-injects `taskId`/`sessionId` from nearest context, so components call `logger.info('x')` without manually threading IDs.

### Instrumentation Points

| Location | Events logged |
|----------|--------------|
| `lib/api-client.ts` | Every request (DEBUG), every error (ERROR) with endpoint + status |
| `contexts/websocket-context.tsx` | Connect, disconnect, reconnect attempts, parse errors |
| `hooks/use-tasks.ts` | Mutation success (INFO), mutation failure (ERROR) |
| `hooks/use-comments.ts` | Add/delete success/failure |
| All page components | Mount/unmount (DEBUG), user-initiated actions (INFO) |
| Error boundaries | Uncaught React errors (ERROR) with component stack |
| Drag-and-drop | Stage change success (INFO), optimistic update failure (ERROR) |
| Session controls | Start/stop/pause actions and results |

---

## 4. Analytics Tab

### Route & Navigation

- New page: `src/app/analytics/page.tsx`
- Added to sidebar nav with a chart icon, label "Analytics"

### Page Layout

**Row 1 — Summary Cards (4 cards)**
| Card | Value | Trend |
|------|-------|-------|
| Total Tokens | input + output sum | vs last 7 days arrow |
| Estimated Cost | tokens × model pricing | — |
| Token Efficiency | tokens ÷ lines written | — |
| Sessions Today | count of sessions started today | — |

**Row 2 — Time-Series (2 charts, side by side)**

*Token Usage Over Time* — `AreaChart`
- X: date, Y: token count
- Two filled series: Input (blue) + Output (purple)
- Toggle buttons: Daily / Weekly / Monthly
- Tooltip shows exact values + ratio

*Tokens per Session* — `BarChart` (horizontal)
- Last 20 sessions
- Each bar colored by task
- Click a bar to jump to session detail

**Row 3 — Breakdown (3 charts)**

*Per Tool Call* — `PieChart` / donut
- Segments: Read, Write, Edit, Bash, Grep, Other
- Center label: total tool calls
- Tooltip: call count + token share %

*Per Language* — `BarChart` (vertical)
- X: file extension (.rs, .ts, .py, .md, other)
- Y: total tokens consumed in calls touching that extension
- Color-coded by language family

*Token Efficiency* — `BarChart` grouped
- Two bars per task: tokens-per-line-written, tokens-per-LOC
- Helps identify which tasks were expensive relative to output

**Row 4 — Session Timeline (full width, on-demand)**

- Dropdown: select any session
- `AreaChart` — X: sequence_no (JSONL position), Y: cumulative tokens
- Color bands mark tool call regions (different color per tool)
- Hover shows: event type, tool name, tokens at that point
- Helps understand where a 5-hour session spent its token budget

### Chart Library

Install `recharts`. Chosen because:
- First-class React component API
- Composable — easy to layer series, tooltips, reference lines
- Dark theme friendly via `stroke`/`fill` props
- No canvas complexity, SSR safe with Next.js

---

## 5. Logs Tab

### Route & Navigation

- New page: `src/app/logs/page.tsx`
- Added to sidebar nav with a terminal/scroll icon, label "Logs"

### Polling Strategy

React Query `useQuery` with `refetchInterval: 5000`. On each fetch, sends `?since=<last_seen_timestamp>` to only fetch new entries. New entries are prepended to the local list.

### Page Layout

**Filter Bar (sticky top)**
- Level pills: ALL / DEBUG / INFO / WARN / ERROR — toggle individually
- Source segment: ALL / Frontend / Backend
- Free-text search input (client-side filter on `message`)
- Task dropdown (populated from tasks API)
- Session dropdown (populated from sessions API)
- "Live" toggle: when on, auto-scrolls and shows new rows immediately

**Log Table** (virtualized with `@tanstack/react-virtual` for large sets)

Columns: Time | Level | Source | Component/Target | Message | Task

- Level badges: DEBUG=gray, INFO=blue-500, WARN=amber-500, ERROR=red-500
- ERROR rows: subtle red-500/10 background + red left border
- Click row: expands inline detail panel with full metadata JSON (syntax highlighted)
- Timestamp: relative ("3s ago") with full ISO on hover
- New rows fade in on poll (no jump)

**"N new logs" Banner**
- When "Live" is off and new logs arrive, a dismissible banner appears at top
- Click to load and scroll to new entries

---

## 6. Frontend Testing

### Install

```bash
cd frontend
npm install -D vitest @vitest/coverage-v8 @testing-library/react \
  @testing-library/user-event @testing-library/jest-dom \
  msw jsdom @playwright/test
```

### Vitest Config (`vitest.config.ts`)

```ts
export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      provider: 'v8',
      threshold: { lines: 80, branches: 80 },
      exclude: ['src/app/**', 'src/components/ui/**', 'src/test/**']
    }
  }
})
```

### MSW Handlers (`src/test/msw/handlers.ts`)

Mock all API endpoints: `/api/tasks`, `/api/sessions`, `/api/logs`, `/api/analytics/*`, `/api/tasks/:id/comments`. Used in both unit and integration tests.

### Test Coverage Plan

**Unit tests** — `src/lib/` and `src/hooks/`:

| File | Test cases |
|------|-----------|
| `lib/api-client.ts` | Success response, 4xx error throws ApiError, 5xx error, custom headers |
| `lib/logger.ts` | Buffer fills and flushes, 10s interval flush, sendBeacon on unload, dedup |
| `lib/utils.ts` | All exported functions |
| `hooks/use-tasks.ts` | List query, create mutation, update mutation, delete mutation, error state |
| `hooks/use-comments.ts` | Add comment, delete comment, optimistic update rollback |
| `hooks/use-logger.ts` | Context injection, all level methods, withContext chaining |
| `hooks/use-task-subscriptions.ts` | WebSocket message handling, subscription lifecycle |

**Component tests** — `src/components/`:

| Component | Test cases |
|-----------|-----------|
| `kanban/kanban-card.tsx` | Renders title/stage badge, correct badge color per stage |
| `kanban/kanban-board.tsx` | All 6 columns render, drag callback fires with correct args |
| `kanban/kanban-column.tsx` | Shows task count, renders children |
| `tasks/task-card.tsx` | Title, priority indicator, click handler |
| `tasks/create-task-dialog.tsx` | Opens on trigger, validates empty title, submits correctly |
| `tasks/task-detail.tsx` | Shows task fields, session list renders |
| `sessions/session-controls.tsx` | Start/stop/pause buttons, disabled states |
| `sessions/session-output.tsx` | Renders output lines, error lines in red |
| `analytics/` components | Data transforms, empty states, loading states |
| `logs/` components | Level badge colors, row expansion, filter callbacks |

**Integration tests** — full page renders with MSW:

| Scenario | Test |
|----------|------|
| Tasks page | Loads, shows MSW tasks, create flow end-to-end |
| Kanban page | Renders all columns, tasks appear in correct column |
| Logs page | Renders table, level filter updates visible rows, poll fires |
| Analytics page | All chart sections render with mocked data, time toggle works |
| Error boundary | API failure shows error state not crash |

**E2E tests** — Playwright (`tests/e2e/`):

| Spec | Scenarios |
|------|-----------|
| `navigation.spec.ts` | All sidebar nav links load correct pages |
| `tasks.spec.ts` | Create task, edit task, delete task |
| `kanban.spec.ts` | Create task, drag card between two columns |
| `sessions.spec.ts` | Start session button visible, output panel appears |
| `analytics.spec.ts` | Page loads, charts visible, time toggle switches data |
| `logs.spec.ts` | Page loads, level filter pills work, row expands |

### Coverage Target

80% lines and branches across `src/` excluding:
- `src/app/` (Next.js page shells — thin wrappers)
- `src/components/ui/` (Shadcn primitives — not our code)
- `src/test/` (test infrastructure itself)

---

## 7. Implementation Order

1. Backend: `token_events` + `session_metrics` migrations
2. Backend: `TokenEventRepository` + JSONL parser changes in `ClaudeManager`
3. Backend: `/api/analytics/*` endpoints
4. Frontend: Logger singleton + hook + instrumentation
5. Frontend: Analytics tab (components + page)
6. Frontend: Logs tab (component + page)
7. Frontend: Test infrastructure setup (Vitest + MSW + Playwright)
8. Frontend: Unit + component tests
9. Frontend: Integration + E2E tests
10. Verify coverage ≥ 80%
