# Interactive Claude Sessions Design

**Date:** 2026-02-28
**Status:** Approved

## Overview

Make Claude sessions interactive via a comment-driven model, fix session lifecycle management, add a New Task button to the Kanban board, and wire up live output streaming.

---

## 1. Comment-Driven Session Interaction

### Model

1. User fills in task description/context, clicks **Start Claude Session**
2. Backend spawns Claude subprocess; live stdout streams to the task detail via WebSocket → shown in a collapsible **"Live Output"** panel
3. When the session completes, the backend extracts Claude's final text response from the JSONL stream and posts it as a comment (`author = "claude"`)
4. User reads the comment, optionally replies in the comment thread
5. A **"Continue Session"** button appears when there is prior Claude comment history; clicking it starts a new session with the full comment thread included as context

### Prompt Building for Follow-up Sessions

```
# Task: {title}

{description}

## Context
{context}

## Conversation History
[Claude]: {claude comment text}
[You]: {user reply text}
[Claude]: ...
```

### JSONL Result Extraction

Claude's `--output-format stream-json` emits a final line:
```json
{"type":"result","subtype":"success","result":"<final assistant text>"}
```

The backend parses this line from the stdout stream, stores the text, and after the process exits creates a comment `(author="claude", content=result_text)`.

---

## 2. Session Lifecycle Management

### Stop on Task Delete

`DELETE /api/tasks/{id}` calls `queue.stop_session(session_id)` (if an active session exists for the task) before the DB cascade delete. Currently the Claude process runs indefinitely as an orphan.

### Auto-stop on Move to Done

`POST /api/tasks/{id}/move` with `stage = "done"` checks for an active session and stops it.
Same logic applies when the Kanban drag-and-drop moves a card to the Done column (uses the same `/move` endpoint).

### Simplified SessionControls UI

Remove non-functional Pause/Resume buttons. New states:

| Session state | Buttons shown |
|---|---|
| None / completed / failed | `Start Session` (+ `Continue Session` if Claude comments exist) |
| Pending | `Starting…` (disabled) |
| Running | `Stop Session` |

---

## 3. New Task Button on Kanban

Add a `+ New Task` button in the Kanban page header. Reuses the existing `CreateTaskDialog` component — no new component needed.

---

## 4. Live Output Panel

A collapsible **"Session Output"** section in the task detail:

- Subscribes to the WebSocket context for `SessionOutput` events filtered to the current `session_id`
- Renders a scrollable, auto-scrolling monospace feed of raw output lines
- Resets when a new session starts
- Shows a pulsing indicator while session is running
- Hidden when no session has ever run for this task

WebSocket flow:
```
ClaudeManager stdout reader
  → broadcast::Sender<SessionOutput>
  → ws/handler.rs (sends to all connected clients)
  → WebSocketContext (frontend)
  → useSessionOutput hook (filters by session_id)
  → LiveOutputPanel component
```

---

## 5. Comments Wiring Fixes

- **Delete button**: Add ✕ button to each comment row; calls existing `DELETE /api/comments/{id}` endpoint
- **Claude author**: Backend already renders `author="claude"` in different colour; ensure new Claude-posted comments are created with `author = "claude"` not `"user"`
- **Reply wiring**: Existing reply input in `CommentThread` already passes `parent_id` — verify it's wired correctly end-to-end

---

## Files Touched

### Backend
- `src/claude/manager.rs` — capture result text, post as claude comment on completion
- `src/api/tasks.rs` — stop session on delete; stop session on move to done
- `src/ws/handler.rs` — filter SessionOutput by session_id per client subscription
- `src/models/comment.rs` / `src/db/comments.rs` — no changes needed (already supports author field)

### Frontend
- `src/app/kanban/page.tsx` — add `+ New Task` button + `CreateTaskDialog`
- `src/components/sessions/session-controls.tsx` — remove Pause/Resume, add Continue Session
- `src/components/tasks/task-detail.tsx` — add `LiveOutputPanel`, wire up WebSocket subscription
- `src/components/tasks/comment-thread.tsx` — add delete button per comment
- `src/hooks/use-sessions.ts` — add `useSessionOutput` hook for WebSocket output
