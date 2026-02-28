# Sprint 5-7: MVP Completion Design

**Date:** 2026-02-28
**Status:** Approved
**Goal:** Complete the MVP with Kanban board, real-time updates, and polish

---

## Current State

| Component | Status |
|-----------|--------|
| Backend API | ✅ Complete (tasks, comments, sessions, WebSocket) |
| Frontend Foundation | ✅ Complete (task list, task detail, comments) |
| Kanban Board | ❌ Missing |
| Real-time Updates | ❌ Missing |
| Session UI | ❌ Missing |
| Polish | ❌ Missing |

---

## 1. Kanban Board View

### Route
- `/kanban` - Full board view
- Link from sidebar

### Layout
```
┌─────────────────────────────────────────────────────────────────────┐
│  Backlog (3)  │  Planning (1)  │  Ready (2)  │  In Progress (1) ... │
├───────────────┼────────────────┼──────────────┼─────────────────────┤
│  ┌──────────┐ │  ┌──────────┐  │  ┌──────────┐│  ┌──────────┐       │
│  │ Task 1   │ │  │ Task 5   │  │  │ Task 3   ││  │ Task 7   │       │
│  └──────────┘ │  └──────────┘  │  └──────────┘│  └──────────┘       │
│  ┌──────────┐ │                │              │                     │
│  │ Task 2   │ │                │              │                     │
│  └──────────┘ │                │              │                     │
│  ┌──────────┐ │                │              │                     │
│  │ Task 4   │ │                │              │                     │
│  └──────────┘ │                │              │                     │
└───────────────┴────────────────┴──────────────┴─────────────────────┘
```

### Features
- 6 columns (Backlog, Planning, Ready, In Progress, Review, Done)
- Task count badge per column
- Drag-and-drop between columns
- Click card → navigate to detail page
- Scrollable columns for overflow

### Components
```
components/kanban/
├── kanban-board.tsx     # Main board with @dnd-kit DndContext
├── kanban-column.tsx    # Droppable column with title + count
└── kanban-card.tsx      # Draggable task card
```

### Dependencies
- `@dnd-kit/core` - Core drag-and-drop
- `@dnd-kit/sortable` - Sorting within columns

---

## 2. Real-time Updates

### WebSocket Connection
- Connect to `ws://localhost:3001/ws` on app mount
- Store connection in React Context
- Auto-reconnect on disconnect

### Message Types
```typescript
// Server → Client
{ type: 'task_updated', task: Task }
{ type: 'task_created', task: Task }
{ type: 'task_deleted', task_id: string }
{ type: 'session_status', session_id: string, status: string }
{ type: 'session_output', session_id: string, output: string }
```

### Implementation
```typescript
// contexts/websocket-context.tsx
const WebSocketContext = createContext<WebSocket | null>(null);

// hooks/use-websocket.ts
export function useWebSocket() {
  return useContext(WebSocketContext);
}

// hooks/use-task-subscriptions.ts
export function useTaskSubscriptions() {
  const ws = useWebSocket();
  const queryClient = useQueryClient();

  useEffect(() => {
    // Listen for task updates and invalidate queries
  }, [ws]);
}
```

### Integration Points
- Task list auto-refreshes when tasks change
- Kanban board updates in real-time
- Session status updates live

---

## 3. Session Management UI

### Task Detail Page Additions

**Session Status Section:**
- Show current session status (none/pending/running/completed/failed)
- Display session ID if active
- Show token usage summary

**Session Controls:**
- "Start Claude Session" button (creates session, spawns Claude)
- "Pause" button (pauses running session)
- "Resume" button (resumes paused session)
- "Stop" button (terminates session)

**Live Output Viewer:**
- Streaming text area showing Claude output
- Auto-scroll to bottom
- Timestamps per line

### Components
```
components/sessions/
├── session-status.tsx   # Status badge with indicator
├── session-controls.tsx # Action buttons
└── session-output.tsx   # Live streaming output
```

### API Endpoints (Backend already has these)
- `POST /api/tasks/:id/sessions` - Start session
- `POST /api/sessions/:id/pause` - Pause
- `POST /api/sessions/:id/resume` - Resume
- `POST /api/sessions/:id/stop` - Stop

---

## 4. Polish

### Loading States
- Skeleton loaders for task cards
- Skeleton for task detail sections
- Loading spinner for session output

### Error Handling
- Toast notifications for errors
- Error boundaries for component failures
- Retry buttons for failed operations

### Empty States
- Empty column message in Kanban
- No sessions message
- No comments message (already done)

### Responsive Design
- Kanban board horizontal scroll on mobile
- Sidebar collapse on small screens
- Touch-friendly drag and drop

---

## Implementation Order

1. **Kanban Board** (core feature)
   - Create Kanban components
   - Add @dnd-kit
   - Wire up to existing API
   - Add route and navigation

2. **WebSocket Integration** (enables real-time)
   - Create WebSocket context
   - Connect on app mount
   - Subscribe to task updates
   - Auto-refresh queries

3. **Session UI** (core feature)
   - Session status component
   - Session controls
   - Output viewer
   - Wire to WebSocket for live updates

4. **Polish** (quality)
   - Loading skeletons
   - Error toasts
   - Empty states
   - Responsive fixes

---

## Files Summary

### New Files
| File | Purpose |
|------|---------|
| `app/kanban/page.tsx` | Kanban board page |
| `components/kanban/kanban-board.tsx` | Board container |
| `components/kanban/kanban-column.tsx` | Droppable column |
| `components/kanban/kanban-card.tsx` | Draggable card |
| `contexts/websocket-context.tsx` | WebSocket provider |
| `hooks/use-websocket.ts` | WebSocket hook |
| `hooks/use-task-subscriptions.ts` | Real-time updates |
| `components/sessions/session-status.tsx` | Status display |
| `components/sessions/session-controls.tsx` | Action buttons |
| `components/sessions/session-output.tsx` | Live output |

### Modified Files
| File | Change |
|------|--------|
| `app/layout.tsx` | Add WebSocket provider |
| `components/layout/sidebar.tsx` | Add Kanban link |
| `components/tasks/task-detail.tsx` | Add session sections |

---

## Dependencies to Add

```bash
npm install @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities
```
