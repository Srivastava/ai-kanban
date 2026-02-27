# AI Kanban - Technical Design Document

**Date:** 2026-02-26
**Status:** Approved
**MVP Scope:** Phase 1 - Core

## 1. Overview

A local-first AI task automation platform that orchestrates multiple Claude CLI agents to work on tasks within a local project directory.

### Key Goals
- Resume Claude sessions (`claude --continue <session_id>`)
- Minimize token usage through structured task context
- Track detailed analytics
- Stage-aware AI prompting (Kanban-based)
- Hyper-modern UI dashboard
- Fully local, no authentication required
- Maximum concurrency: 3 Claude sessions and 3 active tasks

## 2. Technical Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust + Axum + SQLx + SQLite |
| Frontend | Next.js + Tailwind + Shadcn/ui |
| Real-time | WebSockets |
| Claude Integration | Process spawn + stdout parsing |
| Token Analytics | Parse Claude JSONL directly |
| Testing | Backend: cargo test, Frontend: Vitest + Playwright |

## 3. Architecture

**Approach:** Layered Monolith

```
ai-kanban/
├── backend/                    # Rust application
│   ├── src/
│   │   ├── main.rs            # Entry point, Axum server setup
│   │   ├── lib.rs             # Library root
│   │   ├── api/               # HTTP/WebSocket handlers
│   │   │   ├── mod.rs
│   │   │   ├── routes.rs      # Route definitions
│   │   │   ├── tasks.rs       # Task endpoints
│   │   │   ├── sessions.rs    # Claude session endpoints
│   │   │   └── ws.rs          # WebSocket handler
│   │   ├── models/            # Data structures
│   │   │   ├── mod.rs
│   │   │   ├── task.rs        # Task entity
│   │   │   ├── session.rs     # Session entity
│   │   │   └── snapshot.rs    # Snapshot entity
│   │   ├── db/                # Database layer
│   │   │   ├── mod.rs
│   │   │   ├── pool.rs        # SQLite connection pool
│   │   │   └── migrations/    # SQLx migrations
│   │   ├── engine/            # Core business logic
│   │   │   ├── mod.rs
│   │   │   ├── task_engine.rs # Task orchestration
│   │   │   ├── claude_manager.rs # Claude CLI spawning
│   │   │   └── queue.rs       # Concurrency queue (3 sessions max)
│   │   └── analytics/         # Token analytics
│   │       ├── mod.rs
│   │       └── parser.rs      # Claude JSONL parser
│   ├── Cargo.toml
│   └── .sqlx/                 # SQLx query cache
│
├── frontend/                  # Next.js application
│   ├── src/
│   │   ├── app/               # Next.js App Router
│   │   ├── components/        # React components
│   │   │   ├── ui/            # Shadcn components
│   │   │   ├── kanban/        # Kanban board
│   │   │   ├── tasks/         # Task views
│   │   │   └── analytics/     # Analytics charts
│   │   ├── lib/               # Utilities, API client
│   │   └── hooks/             # React hooks (WebSocket, etc.)
│   ├── package.json
│   └── playwright.config.ts
│
├── data/                      # Runtime data (gitignored)
│   ├── tasks/                 # Task memory files
│   └── snapshots/             # Git snapshots
│
└── tests/                     # Integration tests
    ├── e2e/                   # Playwright tests
    └── fixtures/              # Test fixtures
```

## 4. Database Schema

```sql
-- Tasks table: Core entity for the Kanban system
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,              -- UUID v4
    title TEXT NOT NULL,
    description TEXT,
    stage TEXT NOT NULL DEFAULT 'backlog',  -- backlog, planning, ready, in_progress, review, done
    project_path TEXT NOT NULL,       -- Path to the project directory
    session_id TEXT,                  -- Current/last Claude session ID
    priority INTEGER DEFAULT 0,       -- For sorting within stages
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Sessions table: Claude CLI session tracking
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,              -- Claude session ID
    task_id TEXT NOT NULL REFERENCES tasks(id),
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, running, paused, completed, failed
    started_at TIMESTAMP,
    ended_at TIMESTAMP,
    last_snapshot_id TEXT,
    error_message TEXT
);

-- Snapshots table: Git-based task snapshots
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,              -- UUID v4
    task_id TEXT NOT NULL REFERENCES tasks(id),
    session_id TEXT REFERENCES sessions(id),
    commit_hash TEXT,                 -- Git commit hash
    message TEXT,                     -- Snapshot description
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Token usage table: Analytics data
CREATE TABLE token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT REFERENCES tasks(id),
    session_id TEXT REFERENCES sessions(id),
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    model TEXT,                       -- e.g., "claude-sonnet-4-6"
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Stage history: Track task movements for analytics
CREATE TABLE stage_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    from_stage TEXT,
    to_stage TEXT NOT NULL,
    moved_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX idx_tasks_stage ON tasks(stage);
CREATE INDEX idx_tasks_project ON tasks(project_path);
CREATE INDEX idx_sessions_task ON sessions(task_id);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_tokens_task ON token_usage(task_id);
CREATE INDEX idx_tokens_session ON token_usage(session_id);
```

## 5. API Design

### REST Endpoints

```
Tasks
-----
GET    /api/tasks                    # List all tasks (optional ?stage= filter)
GET    /api/tasks/:id                # Get single task with details
POST   /api/tasks                    # Create new task
PATCH  /api/tasks/:id                # Update task (title, description, stage)
DELETE /api/tasks/:id                # Delete task
POST   /api/tasks/:id/move           # Move task to new stage

Sessions
--------
GET    /api/sessions                 # List all sessions
GET    /api/sessions/:id             # Get session details
POST   /api/tasks/:id/sessions       # Start new Claude session for task
POST   /api/sessions/:id/resume      # Resume paused session
POST   /api/sessions/:id/pause       # Pause running session
POST   /api/sessions/:id/stop        # Stop session

Snapshots
---------
GET    /api/tasks/:id/snapshots      # List snapshots for task
POST   /api/tasks/:id/snapshots      # Create manual snapshot
GET    /api/snapshots/:id            # Get snapshot details

Analytics
---------
GET    /api/analytics/overview       # Dashboard stats
GET    /api/analytics/tokens         # Token usage over time
GET    /api/analytics/tasks/:id      # Per-task analytics

Projects
--------
GET    /api/projects                 # List available projects
POST   /api/projects                 # Register project path
```

### WebSocket Protocol

```
ws://localhost:3000/ws

Message Types (Client → Server):
{
  "type": "subscribe_task",
  "task_id": "uuid"
}

Message Types (Server → Client):
{
  "type": "task_updated",
  "task": { ... }
}

{
  "type": "session_output",
  "session_id": "uuid",
  "output": "Claude is thinking..."
}

{
  "type": "session_status",
  "session_id": "uuid",
  "status": "running" | "paused" | "completed"
}

{
  "type": "token_update",
  "task_id": "uuid",
  "input_tokens": 1000,
  "output_tokens": 500
}
```

## 6. Core Engine Design

### Task Engine

```rust
pub struct TaskEngine {
    db: SqlitePool,
    queue: SessionQueue,
    notifier: WebSocketBroadcaster,
}

pub enum Stage {
    Backlog,      // Just created, no Claude interaction
    Planning,     // Claude analyzes and plans (read-only mode)
    Ready,        // Plan approved, ready for implementation
    InProgress,   // Claude is actively implementing
    Review,       // Claude reviews its own work
    Done,         // Completed
}
```

### Claude Manager

```rust
pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, ClaudeSession>>>,
    output_tx: broadcast::Sender<SessionOutput>,
}

// Spawns Claude with stage-appropriate prompts
async fn spawn_claude(&self, task: &Task, stage: &Stage) -> Result<Child> {
    Command::new("claude")
        .arg("--print")
        .arg("--output-format").arg("json")
        .arg("--allowedTools").arg("Read,Write,Edit,Bash,Glob,Grep")
        .arg(&self.build_prompt(task, stage))
        .current_dir(&task.project_path)
        .stdout(Stdio::piped())
        .spawn()
}
```

### Concurrency Queue

```rust
pub struct SessionQueue {
    max_concurrent: usize,          // 3
    active: Arc<RwLock<HashSet<String>>>,
    pending: Arc<Mutex<VecDeque<QueuedTask>>>,
}
```

## 7. Frontend Architecture

### Component Structure

```
frontend/src/
├── app/
│   ├── layout.tsx              # Root layout with providers
│   ├── page.tsx                # Dashboard home
│   ├── tasks/
│   │   ├── page.tsx            # Task list view
│   │   └── [id]/page.tsx       # Single task detail
│   ├── kanban/page.tsx         # Kanban board view
│   └── analytics/page.tsx      # Analytics dashboard
│
├── components/
│   ├── ui/                     # Shadcn components
│   ├── layout/                 # Sidebar, header, stats
│   ├── kanban/                 # Board, column, task-card
│   ├── tasks/                  # Task-detail, session-viewer
│   └── analytics/              # Charts, metrics
│
├── lib/                        # API client, WebSocket, utils
└── hooks/                      # use-tasks, use-websocket, etc.
```

### State Management

- **React Query** for server state (tasks, sessions)
- **React Context** for WebSocket connection
- **@dnd-kit** for drag-and-drop
- **framer-motion** for animations

### Key UI Features

1. Glassmorphism panels
2. Smooth drag-and-drop
3. Real-time indicators (pulsing dot for active sessions)
4. Token meter (animated progress bar)
5. Command palette (Cmd+K)

## 8. Testing Strategy

### Backend Testing

```
backend/tests/
├── unit/                       # Task engine, queue, parser tests
├── integration/                # API endpoints, WebSocket, DB
└── concurrency/                # Session limit, queue, race conditions
```

**Coverage Goal:** 80%+ on core engines

### Frontend Testing

```
tests/
├── e2e/                        # Playwright tests (PRIORITY)
│   ├── dashboard.spec.ts
│   ├── kanban.spec.ts
│   ├── tasks.spec.ts
│   ├── sessions.spec.ts
│   └── analytics.spec.ts
│
├── components/                 # Vitest component tests
└── fixtures/                   # Mock data
```

**Coverage Goal:** All E2E flows covered, critical components unit tested

### Test Commands

```bash
# Backend
cd backend && cargo test

# Frontend
cd frontend && npm run test        # Vitest
cd frontend && npm run test:e2e    # Playwright
```

## 9. Implementation Phases (Phase 1 MVP)

### Sprint 1: Foundation (Backend)

| Step | Task | Tests |
|------|------|-------|
| 1.1 | Initialize Rust project with Axum, SQLx | Smoke test |
| 1.2 | Database schema + migrations | Migration tests |
| 1.3 | Task model + CRUD operations | Unit + DB tests |
| 1.4 | REST API for tasks | Integration tests |

### Sprint 2: Claude Integration

| Step | Task | Tests |
|------|------|-------|
| 2.1 | ClaudeManager: process spawning | Unit tests with mocks |
| 2.2 | Output parsing (session ID, tokens) | Parser unit tests |
| 2.3 | SessionQueue: 3-session limit | Concurrency tests |
| 2.4 | Session REST endpoints | Integration tests |

### Sprint 3: Real-time Layer

| Step | Task | Tests |
|------|------|-------|
| 3.1 | WebSocket server setup | Connection tests |
| 3.2 | Message broadcasting | Delivery tests |
| 3.3 | Subscribe to task updates | Subscription tests |

### Sprint 4: Frontend Foundation

| Step | Task | Tests |
|------|------|-------|
| 4.1 | Initialize Next.js + Shadcn | - |
| 4.2 | API client + React Query setup | Component tests |
| 4.3 | Layout (sidebar, header) | E2E: page loads |
| 4.4 | Task list view | E2E: display, create |

### Sprint 5: Kanban Board

| Step | Task | Tests |
|------|------|-------|
| 5.1 | Kanban columns component | Component tests |
| 5.2 | Task card component | Component tests |
| 5.3 | Drag-and-drop with @dnd-kit | E2E: drag stages |
| 5.4 | WebSocket integration | E2E: real-time |

### Sprint 6: Session Management UI

| Step | Task | Tests |
|------|------|-------|
| 6.1 | Task detail page | E2E: navigation |
| 6.2 | Session viewer (live output) | E2E: output appears |
| 6.3 | Session controls | E2E: all controls |
| 6.4 | Queue indicator | E2E: queue shown |

### Sprint 7: Polish + Testing

| Step | Task | Tests |
|------|------|-------|
| 7.1 | Error handling UI | E2E: error states |
| 7.2 | Loading states + animations | Component tests |
| 7.3 | Full E2E test suite | All flows |
| 7.4 | Performance validation | Concurrency tests |

**Total Steps:** 28

## 10. Future Phases

- **Phase 2:** Snapshots + Task Memory System
- **Phase 3:** Full Analytics Dashboard
- **Phase 4:** Performance Optimization, UI Polish
