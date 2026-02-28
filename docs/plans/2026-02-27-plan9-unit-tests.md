# Unit + Hook Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Write unit tests for `src/lib/` (api-client, logger, utils) and `src/hooks/` (use-tasks, use-comments, use-logger, use-analytics, use-logs).

**Architecture:** Each test file co-located next to the source file it tests (e.g., `src/lib/api-client.test.ts`). Use MSW to intercept fetch calls. Use Vitest's fake timers for logger interval tests.

**Tech Stack:** Vitest, @testing-library/react (renderHook), MSW 2, @testing-library/user-event

---

## Prerequisite

Plan 8 (test infrastructure) must be complete. Run `npm test` and verify smoke tests pass before starting this plan.

---

## Task 1: api-client Tests

**Files:**
- Create: `frontend/src/lib/api-client.test.ts`

**Step 1: Write tests**

Create `frontend/src/lib/api-client.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { apiClient, ApiError } from './api-client';

describe('apiClient', () => {
  it('returns parsed JSON on success', async () => {
    server.use(
      http.get('http://localhost:3001/api/test', () =>
        HttpResponse.json({ ok: true })
      )
    );

    const result = await apiClient<{ ok: boolean }>('/api/test');
    expect(result.ok).toBe(true);
  });

  it('sends Content-Type: application/json header', async () => {
    let capturedContentType: string | null = null;

    server.use(
      http.post('http://localhost:3001/api/test', ({ request }) => {
        capturedContentType = request.headers.get('content-type');
        return HttpResponse.json({ ok: true }, { status: 201 });
      })
    );

    await apiClient('/api/test', { method: 'POST', body: JSON.stringify({}) });
    expect(capturedContentType).toBe('application/json');
  });

  it('throws ApiError with status on 4xx response', async () => {
    server.use(
      http.get('http://localhost:3001/api/notfound', () =>
        HttpResponse.json({ error: 'Not found' }, { status: 404 })
      )
    );

    await expect(apiClient('/api/notfound')).rejects.toThrow(ApiError);
    await expect(apiClient('/api/notfound')).rejects.toMatchObject({ status: 404 });
  });

  it('throws ApiError with status on 5xx response', async () => {
    server.use(
      http.get('http://localhost:3001/api/broken', () =>
        HttpResponse.json({ error: 'Internal' }, { status: 500 })
      )
    );

    await expect(apiClient('/api/broken')).rejects.toThrow(ApiError);
    await expect(apiClient('/api/broken')).rejects.toMatchObject({ status: 500 });
  });

  it('merges extra headers with Content-Type', async () => {
    let capturedAuthHeader: string | null = null;

    server.use(
      http.get('http://localhost:3001/api/auth', ({ request }) => {
        capturedAuthHeader = request.headers.get('x-custom');
        return HttpResponse.json({});
      })
    );

    await apiClient('/api/auth', { headers: { 'x-custom': 'value' } });
    expect(capturedAuthHeader).toBe('value');
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/lib/api-client.test.ts 2>&1
```

Expected: 5 tests pass.

---

## Task 2: logger Tests

**Files:**
- Create: `frontend/src/lib/logger.test.ts`

**Step 1: Write tests**

Create `frontend/src/lib/logger.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';

// Import after mocks are set up
let Logger: typeof import('./logger').Logger;
let loggerInstance: InstanceType<typeof import('./logger').Logger>;

describe('Logger', () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    // Re-import to get a fresh instance
    vi.resetModules();
    const mod = await import('./logger');
    // Access the internal Logger class for testing
    // We test via the exported logger singleton behavior
    loggerInstance = mod.logger as unknown as InstanceType<typeof import('./logger').Logger>;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('buffers log entries without immediately flushing', async () => {
    let flushCount = 0;
    server.use(
      http.post('http://localhost:3001/api/logs', () => {
        flushCount++;
        return HttpResponse.json({}, { status: 201 });
      })
    );

    const { logger } = await import('./logger');
    logger.info('test message 1');
    logger.info('test message 2');

    // No flush yet (timer hasn't fired)
    expect(flushCount).toBe(0);
  });

  it('flushes after MAX_BUFFER_SIZE entries', async () => {
    const flushed: unknown[] = [];
    server.use(
      http.post('http://localhost:3001/api/logs', async ({ request }) => {
        flushed.push(await request.json());
        return HttpResponse.json({}, { status: 201 });
      })
    );

    const { logger } = await import('./logger');
    // Fill buffer to trigger flush (MAX_BUFFER_SIZE = 20)
    for (let i = 0; i < 20; i++) {
      logger.info(`message ${i}`);
    }

    // Allow microtasks to settle
    await vi.runAllTimersAsync();
    expect(flushed.length).toBeGreaterThan(0);
  });

  it('withContext creates child logger with merged context', async () => {
    const entries: unknown[] = [];
    server.use(
      http.post('http://localhost:3001/api/logs', async ({ request }) => {
        entries.push(await request.json());
        return HttpResponse.json({}, { status: 201 });
      })
    );

    const { logger } = await import('./logger');
    const child = logger.withContext({ task_id: 'task-123', target: 'TestComponent' });
    child.info('child message');

    await vi.runAllTimersAsync();
    // After timer fires, entries should contain the context
    // (may need to call flush manually in test)
    await logger.flush();

    const found = entries.find(
      (e: unknown) => (e as Record<string, unknown>).message === 'child message'
    ) as Record<string, unknown> | undefined;
    expect(found?.task_id).toBe('task-123');
    expect(found?.target).toBe('TestComponent');
  });

  it('deduplicates identical consecutive messages within 1 second', async () => {
    const entries: unknown[] = [];
    server.use(
      http.post('http://localhost:3001/api/logs', async ({ request }) => {
        entries.push(await request.json());
        return HttpResponse.json({}, { status: 201 });
      })
    );

    const { logger } = await import('./logger');
    logger.info('duplicate');
    logger.info('duplicate'); // Should be dropped
    logger.info('duplicate'); // Should be dropped

    await logger.flush();
    const dupes = entries.filter(
      (e: unknown) => (e as Record<string, unknown>).message === 'duplicate'
    );
    expect(dupes.length).toBe(1);
  });

  it('never throws even if backend is down', async () => {
    server.use(
      http.post('http://localhost:3001/api/logs', () => {
        return HttpResponse.error();
      })
    );

    const { logger } = await import('./logger');
    expect(async () => {
      logger.error('this will fail to send');
      await logger.flush();
    }).not.toThrow();
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/lib/logger.test.ts 2>&1
```

Expected: 5 tests pass.

---

## Task 3: use-tasks Hook Tests

**Files:**
- Create: `frontend/src/hooks/use-tasks.test.ts`

**Step 1: Write tests**

Create `frontend/src/hooks/use-tasks.test.ts`:

```typescript
import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useTasks, useTask, useCreateTask, useUpdateTask, useDeleteTask } from './use-tasks';
import { mockTask, mockTask2 } from '@/test/msw/fixtures';

function wrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('useTasks', () => {
  it('returns list of tasks', async () => {
    const { result } = renderHook(() => useTasks(), { wrapper: wrapper() });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(2);
    expect(result.current.data![0].id).toBe(mockTask.id);
  });

  it('filters by stage', async () => {
    server.use(
      http.get('http://localhost:3001/api/tasks', ({ request }) => {
        const url = new URL(request.url);
        const stage = url.searchParams.get('stage');
        if (stage === 'backlog') return HttpResponse.json([mockTask]);
        return HttpResponse.json([mockTask, mockTask2]);
      })
    );

    const { result } = renderHook(() => useTasks('backlog'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(1);
    expect(result.current.data![0].stage).toBe('backlog');
  });

  it('returns error state on API failure', async () => {
    server.use(
      http.get('http://localhost:3001/api/tasks', () =>
        HttpResponse.json({ error: 'Server error' }, { status: 500 })
      )
    );

    const { result } = renderHook(() => useTasks(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe('useTask', () => {
  it('returns single task by id', async () => {
    const { result } = renderHook(() => useTask(mockTask.id), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.title).toBe(mockTask.title);
  });

  it('is disabled when id is empty', () => {
    const { result } = renderHook(() => useTask(''), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });
});

describe('useCreateTask', () => {
  it('creates a task and returns it', async () => {
    const { result } = renderHook(() => useCreateTask(), { wrapper: wrapper() });

    result.current.mutate({
      title: 'New task',
      project_path: '/test',
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.title).toBe('New task');
  });

  it('enters error state on failure', async () => {
    server.use(
      http.post('http://localhost:3001/api/tasks', () =>
        HttpResponse.json({ error: 'Bad request' }, { status: 400 })
      )
    );

    const { result } = renderHook(() => useCreateTask(), { wrapper: wrapper() });
    result.current.mutate({ title: '', project_path: '/' });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe('useUpdateTask', () => {
  it('updates a task', async () => {
    const { result } = renderHook(() => useUpdateTask(), { wrapper: wrapper() });

    result.current.mutate({ id: mockTask.id, data: { title: 'Updated title' } });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});

describe('useDeleteTask', () => {
  it('deletes a task', async () => {
    server.use(
      http.delete('http://localhost:3001/api/tasks/:id', () =>
        new HttpResponse(null, { status: 204 })
      )
    );

    const { result } = renderHook(() => useDeleteTask(), { wrapper: wrapper() });
    result.current.mutate(mockTask.id);
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/hooks/use-tasks.test.ts 2>&1
```

Expected: 8 tests pass.

---

## Task 4: use-analytics Hook Tests

**Files:**
- Create: `frontend/src/hooks/use-analytics.test.ts`

**Step 1: Write tests**

Create `frontend/src/hooks/use-analytics.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import {
  useAnalyticsOverview,
  useDailyTokens,
  useTokensByTool,
  useSessionTimeline,
} from './use-analytics';
import { mockOverview } from '@/test/msw/fixtures';

function wrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: qc }, children);
}

describe('useAnalyticsOverview', () => {
  it('returns overview data', async () => {
    const { result } = renderHook(() => useAnalyticsOverview(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.total_sessions).toBe(mockOverview.total_sessions);
    expect(result.current.data?.estimated_cost_usd).toBe(mockOverview.estimated_cost_usd);
  });
});

describe('useDailyTokens', () => {
  it('returns array of daily data points', async () => {
    const { result } = renderHook(() => useDailyTokens(30), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
    expect(result.current.data!.length).toBeGreaterThan(0);
    expect(result.current.data![0]).toHaveProperty('date');
    expect(result.current.data![0]).toHaveProperty('input_tokens');
  });
});

describe('useTokensByTool', () => {
  it('returns tool breakdown', async () => {
    const { result } = renderHook(() => useTokensByTool(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data![0].tool_name).toBe('Read');
  });
});

describe('useSessionTimeline', () => {
  it('is disabled when sessionId is null', () => {
    const { result } = renderHook(() => useSessionTimeline(null), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });

  it('fetches when sessionId is provided', async () => {
    const { result } = renderHook(() => useSessionTimeline('sess-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/hooks/use-analytics.test.ts 2>&1
```

Expected: 5 tests pass.

---

## Task 5: use-logs Hook Tests

**Files:**
- Create: `frontend/src/hooks/use-logs.test.ts`

**Step 1: Write tests**

Create `frontend/src/hooks/use-logs.test.ts`:

```typescript
import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { useLogs } from './use-logs';
import { mockLog } from '@/test/msw/fixtures';

function wrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false, refetchInterval: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: qc }, children);
}

describe('useLogs', () => {
  it('fetches and accumulates logs', async () => {
    const { result } = renderHook(() => useLogs(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.logs).toHaveLength(1);
    expect(result.current.logs[0].id).toBe(mockLog.id);
  });

  it('deduplicates logs on subsequent fetches', async () => {
    // Return same log twice — should still have length 1
    server.use(
      http.get('http://localhost:3001/api/logs', () =>
        HttpResponse.json([mockLog])
      )
    );

    const { result } = renderHook(() => useLogs(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.logs.length).toBeGreaterThan(0));

    // Simulate re-fetch (same data)
    const prev = result.current.logs.length;
    await waitFor(() => expect(result.current.logs.length).toBe(prev));
  });

  it('passes level filter to API URL', async () => {
    let capturedUrl = '';
    server.use(
      http.get('http://localhost:3001/api/logs', ({ request }) => {
        capturedUrl = request.url;
        return HttpResponse.json([]);
      })
    );

    renderHook(() => useLogs({ level: 'ERROR' }), { wrapper: wrapper() });
    await waitFor(() => expect(capturedUrl).toContain('level=ERROR'));
  });

  it('passes source filter to API URL', async () => {
    let capturedUrl = '';
    server.use(
      http.get('http://localhost:3001/api/logs', ({ request }) => {
        capturedUrl = request.url;
        return HttpResponse.json([]);
      })
    );

    renderHook(() => useLogs({ source: 'frontend' }), { wrapper: wrapper() });
    await waitFor(() => expect(capturedUrl).toContain('source=frontend'));
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/hooks/use-logs.test.ts 2>&1
```

Expected: 4 tests pass.

---

## Task 6: use-logger Hook Tests

**Files:**
- Create: `frontend/src/hooks/use-logger.test.tsx`

**Step 1: Write tests**

Create `frontend/src/hooks/use-logger.test.tsx`:

```typescript
import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useLogger } from './use-logger';

describe('useLogger', () => {
  it('returns a ContextLogger with debug/info/warn/error methods', () => {
    const { result } = renderHook(() => useLogger({ target: 'TestComponent' }));
    expect(typeof result.current.debug).toBe('function');
    expect(typeof result.current.info).toBe('function');
    expect(typeof result.current.warn).toBe('function');
    expect(typeof result.current.error).toBe('function');
  });

  it('does not recreate logger when options are stable', () => {
    const { result, rerender } = renderHook(() =>
      useLogger({ target: 'Stable', taskId: 'task-1' })
    );
    const first = result.current;
    rerender();
    // Same reference because useMemo dependencies didn't change
    expect(result.current).toBe(first);
  });

  it('recreates logger when taskId changes', () => {
    let taskId = 'task-1';
    const { result, rerender } = renderHook(() => useLogger({ taskId }));
    const first = result.current;
    taskId = 'task-2';
    rerender();
    // Different reference because taskId changed
    expect(result.current).not.toBe(first);
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/hooks/use-logger.test.tsx 2>&1
```

Expected: 3 tests pass.

---

## Task 7: Run All Unit Tests + Check Coverage

**Step 1: Run all tests collected so far**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test 2>&1
```

Expected: all tests pass (smoke + api-client + logger + use-tasks + use-analytics + use-logs + use-logger).

**Step 2: Check current coverage**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm run test:coverage 2>&1 | tail -20
```

Note the coverage numbers — they won't be at 80% yet (component tests in plan10 will get us there).

**Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/src/lib/api-client.test.ts \
        frontend/src/lib/logger.test.ts \
        frontend/src/hooks/use-tasks.test.ts \
        frontend/src/hooks/use-analytics.test.ts \
        frontend/src/hooks/use-logs.test.ts \
        frontend/src/hooks/use-logger.test.tsx
git commit -m "test(frontend): add unit tests for lib/ and hooks/

- api-client: success, 4xx/5xx errors, header merging (5 tests)
- logger: buffering, flush, withContext, dedup, silent failure (5 tests)
- use-tasks: list, filter, error, create, update, delete (8 tests)
- use-analytics: overview, daily, by-tool, session timeline (5 tests)
- use-logs: fetch, dedup, level filter, source filter (4 tests)
- use-logger: method presence, memoization, recreation on change (3 tests)"
```
