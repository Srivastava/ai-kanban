# Frontend Test Infrastructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Install and configure Vitest + @testing-library/react + MSW + Playwright for the frontend, with a custom test utilities wrapper.

**Architecture:** Vitest runs unit/component tests with jsdom. MSW intercepts API calls at the network level so tests never hit the real backend. Playwright runs E2E tests against the running app. Coverage measured by `@vitest/coverage-v8`.

**Tech Stack:** Vitest 2.x, @testing-library/react 16.x, @testing-library/user-event 14.x, MSW 2.x, jsdom, @playwright/test, @vitejs/plugin-react

---

## Context

- Frontend is at `frontend/` (Next.js 16, React 19, TypeScript)
- `@` alias maps to `frontend/src/` (configured in `tsconfig.json`)
- No tests exist yet anywhere in `frontend/src/`
- All test utilities go in `frontend/src/test/`

---

## Task 1: Install Test Dependencies

**Step 1: Install all packages in one command**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm install -D \
  vitest@2 \
  @vitest/coverage-v8@2 \
  @vitejs/plugin-react \
  @testing-library/react@16 \
  @testing-library/user-event@14 \
  @testing-library/jest-dom@6 \
  msw@2 \
  jsdom \
  @playwright/test
```

**Step 2: Verify installations**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm ls vitest @testing-library/react msw @playwright/test 2>&1 | grep -v "npm warn"
```

Expected: all 4 packages listed at correct versions.

---

## Task 2: Vitest Configuration

**Files:**
- Create: `frontend/vitest.config.ts`

**Step 1: Create vitest config**

Create `frontend/vitest.config.ts`:

```typescript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    exclude: ['node_modules', 'src/components/ui/**', '.next'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov', 'html'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/app/**',           // Next.js page shells — thin wrappers
        'src/components/ui/**', // Shadcn primitives — not our code
        'src/test/**',          // test utilities themselves
        '**/*.d.ts',
        'node_modules/**',
      ],
      thresholds: {
        lines: 80,
        branches: 80,
        functions: 80,
        statements: 80,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
```

**Step 2: Add test scripts to package.json**

Open `frontend/package.json`. Add to the `"scripts"` section:

```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage",
"test:e2e": "playwright test"
```

**Step 3: Run vitest to verify config is valid**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx vitest run 2>&1 | head -10
```

Expected: "No test files found" or similar — that's fine, config is valid.

---

## Task 3: Test Setup File

**Files:**
- Create: `frontend/src/test/setup.ts`

**Step 1: Create setup file**

Create `frontend/src/test/setup.ts`:

```typescript
import '@testing-library/jest-dom';
import { beforeAll, afterEach, afterAll, vi } from 'vitest';
import { server } from './msw/server';

// Start MSW server before all tests
beforeAll(() => server.listen({ onUnhandledRequest: 'warn' }));

// Reset handlers after each test (so per-test overrides don't bleed)
afterEach(() => server.resetHandlers());

// Stop server after all tests
afterAll(() => server.close());

// Mock Next.js router
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
  }),
  usePathname: () => '/',
  useSearchParams: () => new URLSearchParams(),
}));

// Mock next/font/google (used in layout.tsx — causes issues in tests)
vi.mock('next/font/google', () => ({
  Geist: () => ({ variable: '--font-geist-sans', className: 'geist-sans' }),
  Geist_Mono: () => ({ variable: '--font-geist-mono', className: 'geist-mono' }),
}));

// Suppress console.error for expected React warnings in tests
const originalConsoleError = console.error;
beforeAll(() => {
  console.error = (...args: unknown[]) => {
    const msg = args[0]?.toString() ?? '';
    // Suppress React 19 act() warnings in test output
    if (msg.includes('act(') || msg.includes('ReactDOMTestUtils')) return;
    originalConsoleError(...args);
  };
});
afterAll(() => {
  console.error = originalConsoleError;
});
```

---

## Task 4: MSW Handlers

**Files:**
- Create: `frontend/src/test/msw/server.ts`
- Create: `frontend/src/test/msw/handlers.ts`
- Create: `frontend/src/test/msw/fixtures.ts`

**Step 1: Create fixtures**

Create `frontend/src/test/msw/fixtures.ts`:

```typescript
import type { Task } from '@/types/task';
import type { LogEntry } from '@/types/log';
import type { AnalyticsOverview } from '@/types/analytics';

export const mockTask: Task = {
  id: 'task-123',
  title: 'Test task',
  description: 'A test task description',
  context: null,
  stage: 'backlog',
  project_path: '/test/project',
  session_id: null,
  priority: 0,
  created_at: '2026-02-27T10:00:00Z',
  updated_at: '2026-02-27T10:00:00Z',
};

export const mockTask2: Task = {
  ...mockTask,
  id: 'task-456',
  title: 'In-progress task',
  stage: 'in_progress',
};

export const mockLog: LogEntry = {
  id: 1,
  timestamp: '2026-02-27T10:00:00Z',
  level: 'INFO',
  message: 'Test log message',
  target: 'test:component',
  source: 'frontend',
  task_id: null,
  session_id: null,
  metadata: null,
  created_at: '2026-02-27T10:00:00Z',
};

export const mockOverview: AnalyticsOverview = {
  total_input_tokens: 150000,
  total_output_tokens: 45000,
  total_sessions: 12,
  total_tasks_with_sessions: 5,
  estimated_cost_usd: 1.125,
  active_sessions_today: 2,
};
```

**Step 2: Create handlers**

Create `frontend/src/test/msw/handlers.ts`:

```typescript
import { http, HttpResponse } from 'msw';
import { mockTask, mockTask2, mockLog, mockOverview } from './fixtures';

const API_BASE = 'http://localhost:3001';

export const handlers = [
  // Tasks
  http.get(`${API_BASE}/api/tasks`, () => {
    return HttpResponse.json([mockTask, mockTask2]);
  }),
  http.get(`${API_BASE}/api/tasks/:id`, ({ params }) => {
    if (params.id === mockTask.id) return HttpResponse.json(mockTask);
    return HttpResponse.json({ error: 'Not found' }, { status: 404 });
  }),
  http.post(`${API_BASE}/api/tasks`, async ({ request }) => {
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json(
      { ...mockTask, id: 'new-task-id', title: body.title as string },
      { status: 201 }
    );
  }),
  http.patch(`${API_BASE}/api/tasks/:id`, async ({ params, request }) => {
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json({ ...mockTask, id: params.id as string, ...body });
  }),
  http.delete(`${API_BASE}/api/tasks/:id`, () => {
    return new HttpResponse(null, { status: 204 });
  }),
  http.post(`${API_BASE}/api/tasks/:id/move`, async ({ params, request }) => {
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json({ ...mockTask, id: params.id as string, stage: body.stage });
  }),

  // Logs
  http.get(`${API_BASE}/api/logs`, () => {
    return HttpResponse.json([mockLog]);
  }),
  http.post(`${API_BASE}/api/logs`, () => {
    return HttpResponse.json({ ...mockLog, id: 2 }, { status: 201 });
  }),

  // Analytics
  http.get(`${API_BASE}/api/analytics/overview`, () => {
    return HttpResponse.json(mockOverview);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/daily`, () => {
    return HttpResponse.json([
      { date: '2026-02-25', input_tokens: 10000, output_tokens: 3000 },
      { date: '2026-02-26', input_tokens: 25000, output_tokens: 7500 },
      { date: '2026-02-27', input_tokens: 15000, output_tokens: 4500 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/weekly`, () => {
    return HttpResponse.json([
      { week_start: '2026-02-16', input_tokens: 80000, output_tokens: 24000 },
      { week_start: '2026-02-23', input_tokens: 50000, output_tokens: 15000 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/monthly`, () => {
    return HttpResponse.json([
      { month: '2026-01', input_tokens: 200000, output_tokens: 60000 },
      { month: '2026-02', input_tokens: 150000, output_tokens: 45000 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-task`, () => {
    return HttpResponse.json([
      { task_id: 'task-123', task_title: 'Test task', input_tokens: 80000, output_tokens: 24000, total_tokens: 104000 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-session`, () => {
    return HttpResponse.json([]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-tool`, () => {
    return HttpResponse.json([
      { tool_name: 'Read', input_tokens: 50000, output_tokens: 0, call_count: 120 },
      { tool_name: 'Write', input_tokens: 0, output_tokens: 24000, call_count: 45 },
      { tool_name: 'Bash', input_tokens: 10000, output_tokens: 5000, call_count: 30 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-language`, () => {
    return HttpResponse.json([
      { file_ext: '.rs', input_tokens: 30000, output_tokens: 12000, call_count: 80 },
      { file_ext: '.ts', input_tokens: 20000, output_tokens: 8000, call_count: 60 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/efficiency`, () => {
    return HttpResponse.json([]);
  }),
  http.get(`${API_BASE}/api/analytics/sessions/:id/timeline`, () => {
    return HttpResponse.json([]);
  }),

  // Sessions
  http.get(`${API_BASE}/api/sessions`, () => {
    return HttpResponse.json([]);
  }),
  http.post(`${API_BASE}/api/tasks/:id/sessions`, () => {
    return HttpResponse.json({ id: 'sess-123', task_id: 'task-123', status: 'running' }, { status: 201 });
  }),

  // Comments
  http.get(`${API_BASE}/api/tasks/:id/comments`, () => {
    return HttpResponse.json([]);
  }),
];
```

**Step 3: Create MSW server**

Create `frontend/src/test/msw/server.ts`:

```typescript
import { setupServer } from 'msw/node';
import { handlers } from './handlers';

export const server = setupServer(...handlers);
```

---

## Task 5: Test Utilities Wrapper

**Files:**
- Create: `frontend/src/test/utils.tsx`

**Step 1: Create renderWithProviders**

Create `frontend/src/test/utils.tsx`:

```tsx
import { render, type RenderOptions } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactElement, ReactNode } from 'react';

/** Create a fresh QueryClient for each test — prevents state bleed */
function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,          // Don't retry on failure in tests
        staleTime: Infinity,   // Don't refetch during test
      },
      mutations: {
        retry: false,
      },
    },
  });
}

interface WrapperProps {
  children: ReactNode;
}

/** Wraps rendered component with all required providers */
function AllProviders({ children }: WrapperProps) {
  const queryClient = createTestQueryClient();
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}

/** Drop-in replacement for RTL's render() — includes all providers */
function renderWithProviders(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
) {
  return render(ui, { wrapper: AllProviders, ...options });
}

// Re-export everything from RTL so tests only need to import from this file
export * from '@testing-library/react';
export { renderWithProviders };
```

---

## Task 6: Playwright Configuration

**Files:**
- Create: `frontend/playwright.config.ts`

**Step 1: Create Playwright config**

Create `frontend/playwright.config.ts`:

```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './src/test/e2e',
  fullyParallel: true,
  retries: 0,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  // Don't start dev server in CI — assume it's running
  // For local development, uncomment:
  // webServer: {
  //   command: 'npm run dev',
  //   url: 'http://localhost:3000',
  //   reuseExistingServer: !process.env.CI,
  // },
});
```

**Step 2: Install Playwright browsers**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx playwright install chromium 2>&1 | tail -5
```

Expected: "Chromium ... downloaded"

---

## Task 7: Smoke Test — Verify Everything Wires Up

**Files:**
- Create: `frontend/src/test/smoke.test.ts`

**Step 1: Create smoke test**

Create `frontend/src/test/smoke.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';

describe('test infrastructure', () => {
  it('vitest runs', () => {
    expect(1 + 1).toBe(2);
  });

  it('MSW server is configured', async () => {
    const response = await fetch('http://localhost:3001/api/tasks');
    expect(response.ok).toBe(true);
    const data = await response.json();
    expect(Array.isArray(data)).toBe(true);
    expect(data[0].id).toBe('task-123');
  });
});
```

**Step 2: Run smoke test**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test 2>&1
```

Expected:
```
✓ src/test/smoke.test.ts (2)
  ✓ test infrastructure > vitest runs
  ✓ test infrastructure > MSW server is configured

Test Files  1 passed (1)
Tests       2 passed (2)
```

**Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/vitest.config.ts \
        frontend/playwright.config.ts \
        frontend/src/test/ \
        frontend/package.json
git commit -m "test: add Vitest + MSW + Playwright test infrastructure

- vitest.config.ts: jsdom, @vitejs/plugin-react, coverage v8 with 80% threshold
- MSW server with handlers for all API endpoints (tasks, logs, analytics, sessions)
- Mock fixtures: mockTask, mockLog, mockOverview
- renderWithProviders: RTL wrapper with fresh QueryClient per test
- Next.js navigation mocked (useRouter, usePathname, useSearchParams)
- Playwright config for E2E against localhost:3000
- Smoke test verifies MSW intercepts correctly"
```
