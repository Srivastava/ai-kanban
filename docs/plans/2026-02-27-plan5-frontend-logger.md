# Frontend Logger Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a structured frontend logger that batches browser log events and ships them to `/api/logs`, then instrument key files to emit logs automatically.

**Architecture:** Singleton `logger` object with `debug/info/warn/error` methods. Buffers entries in memory, flushes every 10s or when buffer reaches 20 entries, and on `beforeunload` via `navigator.sendBeacon`. A `useLogger` hook wraps it and auto-injects `taskId`/`sessionId` context. Flush failures are silently dropped — logging never breaks the UI.

**Tech Stack:** TypeScript, Next.js, native `fetch` / `navigator.sendBeacon`

---

## Context

Key files to instrument after creating the logger:
- `frontend/src/lib/api-client.ts` — log every request + error
- `frontend/src/contexts/websocket-context.tsx` — connect/disconnect/errors
- `frontend/src/hooks/use-tasks.ts` — mutation success/failure

Backend log endpoint: `POST http://localhost:3001/api/logs`
Payload: `{ level, message, source, task_id?, session_id?, metadata? }`

---

## Task 1: Logger Singleton

**Files:**
- Create: `frontend/src/lib/logger.ts`

**Step 1: Create the logger**

Create `frontend/src/lib/logger.ts`:

```typescript
const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';
const FLUSH_INTERVAL_MS = 10_000;
const MAX_BUFFER_SIZE = 20;

interface LogEntry {
  level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
  message: string;
  source: 'frontend';
  target?: string;
  task_id?: string;
  session_id?: string;
  metadata?: Record<string, unknown>;
}

interface LogContext {
  task_id?: string;
  session_id?: string;
  target?: string;
}

class Logger {
  private buffer: LogEntry[] = [];
  private flushTimer: ReturnType<typeof setInterval> | null = null;
  private context: LogContext = {};
  private lastMessage = '';
  private lastMessageTime = 0;

  constructor() {
    if (typeof window !== 'undefined') {
      this.flushTimer = setInterval(() => this.flush(), FLUSH_INTERVAL_MS);
      window.addEventListener('beforeunload', () => this.flushSync());
    }
  }

  /** Set persistent context injected into every log entry */
  setContext(ctx: LogContext) {
    this.context = { ...this.context, ...ctx };
  }

  /** Return a child logger with merged context (does not mutate this instance) */
  withContext(ctx: LogContext): ContextLogger {
    return new ContextLogger(this, { ...this.context, ...ctx });
  }

  debug(message: string, metadata?: Record<string, unknown>) {
    this.log('DEBUG', message, metadata);
  }

  info(message: string, metadata?: Record<string, unknown>) {
    this.log('INFO', message, metadata);
  }

  warn(message: string, metadata?: Record<string, unknown>) {
    this.log('WARN', message, metadata);
  }

  error(message: string, metadata?: Record<string, unknown>) {
    this.log('ERROR', message, metadata);
  }

  log(
    level: LogEntry['level'],
    message: string,
    metadata?: Record<string, unknown>,
    ctx?: LogContext
  ) {
    // Deduplicate identical consecutive messages within 1 second
    const now = Date.now();
    if (message === this.lastMessage && now - this.lastMessageTime < 1000) {
      return;
    }
    this.lastMessage = message;
    this.lastMessageTime = now;

    const merged = { ...this.context, ...ctx };
    const entry: LogEntry = {
      level,
      message,
      source: 'frontend',
      target: merged.target,
      task_id: merged.task_id,
      session_id: merged.session_id,
      metadata,
    };

    this.buffer.push(entry);

    if (this.buffer.length >= MAX_BUFFER_SIZE) {
      this.flush();
    }
  }

  async flush() {
    if (this.buffer.length === 0) return;

    const entries = this.buffer.splice(0, this.buffer.length);
    try {
      // Send all entries; backend accepts one at a time, so batch sequentially
      await Promise.allSettled(
        entries.map((entry) =>
          fetch(`${API_BASE}/api/logs`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(entry),
          })
        )
      );
    } catch {
      // Silent drop — logging must never break the UI
    }
  }

  /** Synchronous flush using sendBeacon for beforeunload */
  private flushSync() {
    if (this.buffer.length === 0) return;
    const entries = this.buffer.splice(0, this.buffer.length);
    for (const entry of entries) {
      try {
        navigator.sendBeacon(
          `${API_BASE}/api/logs`,
          new Blob([JSON.stringify(entry)], { type: 'application/json' })
        );
      } catch {
        // Silent drop
      }
    }
  }

  destroy() {
    if (this.flushTimer) {
      clearInterval(this.flushTimer);
    }
  }
}

class ContextLogger {
  constructor(
    private parent: Logger,
    private ctx: LogContext
  ) {}

  debug(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('DEBUG', message, metadata, this.ctx);
  }

  info(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('INFO', message, metadata, this.ctx);
  }

  warn(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('WARN', message, metadata, this.ctx);
  }

  error(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('ERROR', message, metadata, this.ctx);
  }
}

// Export singleton — created once on import
export const logger = new Logger();
export type { LogContext, ContextLogger };
```

**Step 2: Verify TypeScript compiles**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

---

## Task 2: useLogger Hook

**Files:**
- Create: `frontend/src/hooks/use-logger.ts`

**Step 1: Create the hook**

Create `frontend/src/hooks/use-logger.ts`:

```typescript
'use client';

import { useMemo } from 'react';
import { logger } from '@/lib/logger';
import type { ContextLogger } from '@/lib/logger';

interface UseLoggerOptions {
  target?: string;
  taskId?: string;
  sessionId?: string;
}

/**
 * Returns a context-bound logger.
 * Automatically injects target/taskId/sessionId so components
 * don't need to pass them manually.
 */
export function useLogger(options: UseLoggerOptions = {}): ContextLogger {
  return useMemo(
    () =>
      logger.withContext({
        target: options.target,
        task_id: options.taskId,
        session_id: options.sessionId,
      }),
    // Re-create only when IDs change
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [options.target, options.taskId, options.sessionId]
  );
}
```

**Step 2: Verify TypeScript compiles**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit 2>&1 | head -20
```

---

## Task 3: Instrument api-client.ts

**Files:**
- Modify: `frontend/src/lib/api-client.ts`

**Step 1: Add logging to the API client**

Open `frontend/src/lib/api-client.ts`. Replace the file with:

```typescript
import { logger } from '@/lib/logger';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = 'ApiError';
  }
}

export async function apiClient<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  logger.debug(`API request: ${options?.method ?? 'GET'} ${endpoint}`, {
    method: options?.method ?? 'GET',
    endpoint,
  });

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const errorText = await response.text().catch(() => 'Unknown error');
    logger.error(`API error: ${options?.method ?? 'GET'} ${endpoint} → ${response.status}`, {
      endpoint,
      status: response.status,
      message: errorText,
    });
    throw new ApiError(response.status, errorText);
  }

  return response.json();
}
```

---

## Task 4: Instrument WebSocket Context

**Files:**
- Modify: `frontend/src/contexts/websocket-context.tsx`

**Step 1: Add logger calls**

Open `frontend/src/contexts/websocket-context.tsx`. Add at top:

```typescript
import { logger } from '@/lib/logger';
```

In the `connect` function:

```typescript
socket.onopen = () => {
  logger.info('WebSocket connected', { url: WS_URL });
  setStatus('connected');
  setWs(socket);
};

socket.onclose = () => {
  logger.warn('WebSocket disconnected, reconnecting in 3s');
  setStatus('disconnected');
  setWs(null);
  setTimeout(connect, 3000);
};

socket.onerror = (error) => {
  logger.error('WebSocket error', { error: String(error) });
  console.error('WebSocket error:', error);
};

// In onmessage, catch block:
} catch {
  logger.error('Failed to parse WebSocket message');
  console.error('Failed to parse WebSocket message');
}
```

---

## Task 5: Instrument use-tasks.ts

**Files:**
- Modify: `frontend/src/hooks/use-tasks.ts`

**Step 1: Add logger to mutations**

Open `frontend/src/hooks/use-tasks.ts`. Add at top:

```typescript
import { logger } from '@/lib/logger';
```

Update `useCreateTask`:

```typescript
return useMutation({
  mutationFn: (data: CreateTask) =>
    apiClient<Task>('/api/tasks', { method: 'POST', body: JSON.stringify(data) }),
  onSuccess: (task) => {
    logger.info('Task created', { taskId: task.id, title: task.title });
    queryClient.invalidateQueries({ queryKey: ['tasks'] });
  },
  onError: (error) => {
    logger.error('Failed to create task', { error: String(error) });
  },
});
```

Update `useUpdateTask`:

```typescript
onSuccess: (task, { id }) => {
  logger.info('Task updated', { taskId: id, stage: task.stage });
  queryClient.invalidateQueries({ queryKey: ['tasks'] });
  queryClient.invalidateQueries({ queryKey: ['tasks', id] });
},
onError: (error, { id }) => {
  logger.error('Failed to update task', { taskId: id, error: String(error) });
},
```

Update `useDeleteTask`:

```typescript
onSuccess: (_, id) => {
  logger.info('Task deleted', { taskId: id });
  queryClient.invalidateQueries({ queryKey: ['tasks'] });
},
onError: (error, id) => {
  logger.error('Failed to delete task', { taskId: id, error: String(error) });
},
```

---

## Task 6: Add Error Boundary

**Files:**
- Create: `frontend/src/components/error-boundary.tsx`
- Modify: `frontend/src/app/layout.tsx`

**Step 1: Create error boundary**

Create `frontend/src/components/error-boundary.tsx`:

```tsx
'use client';

import React from 'react';
import { logger } from '@/lib/logger';

interface Props {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    logger.error('Uncaught React error', {
      error: error.message,
      stack: error.stack?.slice(0, 500),
      componentStack: info.componentStack?.slice(0, 500),
    });
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback ?? (
          <div className="flex min-h-screen items-center justify-center">
            <div className="text-center space-y-2">
              <p className="text-destructive font-medium">Something went wrong</p>
              <p className="text-muted-foreground text-sm">{this.state.error?.message}</p>
              <button
                onClick={() => this.setState({ hasError: false })}
                className="text-sm text-primary underline"
              >
                Try again
              </button>
            </div>
          </div>
        )
      );
    }
    return this.props.children;
  }
}
```

**Step 2: Wrap app in layout.tsx**

Open `frontend/src/app/layout.tsx`. Add the import and wrap:

```tsx
import { ErrorBoundary } from '@/components/error-boundary';

// In RootLayout body:
<ErrorBoundary>
  <WebSocketProvider>
    <QueryProvider>{children}</QueryProvider>
  </WebSocketProvider>
</ErrorBoundary>
```

**Step 3: Verify build**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

**Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/src/lib/logger.ts \
        frontend/src/hooks/use-logger.ts \
        frontend/src/lib/api-client.ts \
        frontend/src/contexts/websocket-context.tsx \
        frontend/src/hooks/use-tasks.ts \
        frontend/src/components/error-boundary.tsx \
        frontend/src/app/layout.tsx
git commit -m "feat(frontend): add structured logger with backend shipping

- Logger singleton: batches entries, flushes every 10s or at 20 entries
- sendBeacon flush on page unload
- useLogger hook with auto-injected task/session context
- Instrumented: api-client (all requests/errors), websocket context, use-tasks mutations
- ErrorBoundary component catches and logs uncaught React errors"
```
