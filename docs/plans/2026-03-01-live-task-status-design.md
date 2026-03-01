# Live Task Status + Stage Progression Design

**Date:** 2026-03-01
**Status:** Approved

## Problem

1. `manager.rs` outputs raw JSONL lines to the broadcast channel — the Live Output Panel displays unreadable JSON blobs and often misses output entirely (session_id not yet linked to task when panel mounts)
2. No indicator when Claude is running but silent (thinking between tool calls)
3. Task stage never changes automatically — user can't tell where Claude is in the work

## Solution Overview

Three coordinated changes:
- **Unified `ClaudeEvent` channel** replaces narrow `SessionOutput` struct
- **JSONL → human-readable parsing** added to `jsonl_parser.rs`
- **Stage auto-progression** driven by JSONL event detection in `manager.rs`
- **Heartbeat** from backend every 5s while session is active
- **Frontend** reads heartbeat for animated running indicator; reads `task_updated` WS events to update task stage badge in real-time

## Backend Architecture

### New `ClaudeEvent` enum

Location: `backend/src/claude/mod.rs` (or new `backend/src/claude/events.rs`)

```rust
#[derive(Debug, Clone)]
pub enum ClaudeEvent {
    Output {
        session_id: String,
        text: String,
        is_error: bool,
    },
    Heartbeat {
        session_id: String,
        elapsed_secs: u64,
    },
    SessionStatus {
        session_id: String,
        status: String,
    },
    TaskStageChanged {
        task_id: String,
        stage: String,
        task: serde_json::Value, // full serialized task for WS broadcast
    },
}
```

`ClaudeManager.output_tx` changes type from `broadcast::Sender<SessionOutput>` to `broadcast::Sender<ClaudeEvent>`.

### `manager.rs` event emissions

| Moment | Event emitted | DB update |
|--------|--------------|-----------|
| Session start | `TaskStageChanged(planning)` | `task.stage = "planning"` |
| First `tool_use` in JSONL | `TaskStageChanged(in_progress)` | `task.stage = "in_progress"` |
| Every parsed JSONL line | `Output { text: human_readable }` | none |
| Every 5s while active | `Heartbeat { elapsed_secs }` | none |
| Session complete (exit_ok) | `SessionStatus(completed)`, `TaskStageChanged(review)` | `session.status`, `task.stage = "review"` |
| Session failed | `SessionStatus(failed)` | `session.status` only, no stage change |

Stage progression always overwrites (no "only advance forward" logic) — session start always sets planning, first tool always sets in_progress, completion always sets review.

Per-session state tracked in the spawn_blocking reader: `first_tool_seen: bool = false`.

### `jsonl_parser.rs` — `parse_for_display()`

New function that maps Claude's stream-json events to human-readable strings:

| JSONL type | Display string |
|-----------|----------------|
| `assistant` with text content | `🤔 [first 120 chars]` |
| `assistant` + tool_use `Read` | `📖 Read: path/to/file` |
| `assistant` + tool_use `Write`/`Edit`/`NotebookEdit` | `✏️ Write: path/to/file` |
| `assistant` + tool_use `Bash` | `⚡ Bash: [command preview 80 chars]` |
| `assistant` + tool_use `Glob`/`Grep` | `🔍 [ToolName]: [pattern]` |
| `assistant` + tool_use (other) | `🔧 [ToolName]: [first input arg]` |
| `result` subtype `success` | `✅ Session complete` |
| `result` subtype `error` | `❌ Error: [message]` |
| `system` | `None` (skip) |

Returns `Option<String>` — `None` means skip (don't emit Output event for this line).

Also returns whether a `tool_use` was detected (for stage progression).

### `ws/handler.rs` changes

Subscribe to `ClaudeEvent` channel. Map to `ServerMessage`:

| `ClaudeEvent` | `ServerMessage` |
|--------------|----------------|
| `Output` | `session_output { session_id, output, is_error }` |
| `Heartbeat` | `session_heartbeat { session_id, elapsed_secs }` |
| `SessionStatus` | `session_status { session_id, status }` |
| `TaskStageChanged` | `task_updated { task }` |

Filtering: `Output`, `Heartbeat`, `SessionStatus` filtered by subscribed session_id. `task_updated` broadcast to all clients.

### `ws/messages.rs` additions

Add new `ServerMessage` variants:
```rust
#[serde(rename = "session_heartbeat")]
SessionHeartbeat { session_id: String, elapsed_secs: u64 },
```

(`task_updated` variant already exists but was never sent — now wired up.)

## Frontend Architecture

### `LiveOutputPanel` changes

- Subscribe to `session_heartbeat` → store `{ elapsedSecs, receivedAt }` state
- Header: show `● Live · Running Xs` (where X = elapsed_secs from last heartbeat + ms since it arrived)
- If no heartbeat received for >8s while status is running → show `● Live · Waiting...`
- Lines rendered are now human-readable text (not raw JSONL) — no parsing needed in frontend

### New `useTaskUpdates` hook or inline in `WebSocketProvider`

Global WS subscription to `task_updated`:
```ts
subscribe('task_updated', (data) => {
    const task = (data as { task: Task }).task;
    queryClient.setQueryData(['tasks', task.id], task);
    queryClient.invalidateQueries({ queryKey: ['tasks'] }); // updates kanban board
});
```

This causes the stage badge in `TaskDetail` and the kanban column to update in real-time without a page refresh.

### No changes needed to `task-detail.tsx`

The stage badge already reads `task.stage` — once the query cache updates, it re-renders automatically.

## Data Flow

```
Claude CLI stdout
    → manager.rs spawn_blocking reader
        → parse_for_display()       → ClaudeEvent::Output (human text)
        → detect tool_use           → ClaudeEvent::TaskStageChanged (planning→in_progress)
        → parse_jsonl_line()        → token event DB record
    → ClaudeEvent broadcast channel
        → ws/handler.rs
            → session_output    → LiveOutputPanel (readable lines)
            → session_heartbeat → LiveOutputPanel (running indicator)
            → task_updated      → useTaskUpdates (stage badge, kanban)
```

## Files Changed

**Backend:**
- `backend/src/claude/manager.rs` — unified event channel, stage progression, heartbeat
- `backend/src/claude/jsonl_parser.rs` — add `parse_for_display()`
- `backend/src/ws/handler.rs` — subscribe to `ClaudeEvent`, map to ServerMessage
- `backend/src/ws/messages.rs` — add `SessionHeartbeat` variant, wire `TaskUpdated`
- `backend/src/claude/mod.rs` — export `ClaudeEvent` enum

**Frontend:**
- `frontend/src/components/sessions/live-output-panel.tsx` — heartbeat indicator
- `frontend/src/contexts/websocket-context.tsx` — global `task_updated` handler with queryClient
- `frontend/src/types/session.ts` (if exists) — add heartbeat message type
