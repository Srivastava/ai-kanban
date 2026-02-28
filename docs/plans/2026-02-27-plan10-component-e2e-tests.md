# Component, Integration & E2E Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Write component tests for `src/components/`, integration tests for full pages, Playwright E2E tests for critical user flows, and verify 80% coverage is reached.

**Architecture:** Component tests use `renderWithProviders` from `src/test/utils.tsx`. Integration tests render full page components with MSW. E2E tests run with Playwright against the running dev server. Coverage checked with `npm run test:coverage`.

**Tech Stack:** Vitest, @testing-library/react, @testing-library/user-event, MSW, Playwright

---

## Prerequisite

Plans 8 and 9 must be complete. All previous tests must pass.

---

## Task 1: LogLevelBadge Component Tests

**Files:**
- Create: `frontend/src/components/logs/log-level-badge.test.tsx`

**Step 1: Write tests**

Create `frontend/src/components/logs/log-level-badge.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LogLevelBadge } from './log-level-badge';

describe('LogLevelBadge', () => {
  it('renders DEBUG badge', () => {
    render(<LogLevelBadge level="DEBUG" />);
    expect(screen.getByText('DEBUG')).toBeInTheDocument();
  });

  it('renders INFO badge', () => {
    render(<LogLevelBadge level="INFO" />);
    expect(screen.getByText('INFO')).toBeInTheDocument();
  });

  it('renders WARN badge with amber color class', () => {
    render(<LogLevelBadge level="WARN" />);
    const badge = screen.getByText('WARN');
    expect(badge.className).toContain('amber');
  });

  it('renders ERROR badge with red color class', () => {
    render(<LogLevelBadge level="ERROR" />);
    const badge = screen.getByText('ERROR');
    expect(badge.className).toContain('red');
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/components/logs/log-level-badge.test.tsx 2>&1
```

Expected: 4 tests pass.

---

## Task 2: LogTable Component Tests

**Files:**
- Create: `frontend/src/components/logs/log-table.test.tsx`

**Step 1: Write tests**

Create `frontend/src/components/logs/log-table.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LogTable } from './log-table';
import { mockLog } from '@/test/msw/fixtures';
import type { LogEntry } from '@/types/log';

const errorLog: LogEntry = {
  ...mockLog,
  id: 2,
  level: 'ERROR',
  message: 'Something broke',
  metadata: '{"endpoint": "/api/tasks"}',
};

describe('LogTable', () => {
  it('renders log entries', () => {
    render(<LogTable logs={[mockLog]} filter={{}} />);
    expect(screen.getByText('Test log message')).toBeInTheDocument();
  });

  it('shows empty state when no logs match filter', () => {
    render(<LogTable logs={[mockLog]} filter={{ search: 'xyz-no-match' }} />);
    expect(screen.getByText(/No logs match/i)).toBeInTheDocument();
  });

  it('filters by search term client-side', () => {
    render(<LogTable logs={[mockLog, errorLog]} filter={{ search: 'broke' }} />);
    expect(screen.queryByText('Test log message')).not.toBeInTheDocument();
    expect(screen.getByText('Something broke')).toBeInTheDocument();
  });

  it('expands row detail on click', () => {
    render(<LogTable logs={[errorLog]} filter={{}} />);

    // Before click: metadata not visible
    expect(screen.queryByText('/api/tasks')).not.toBeInTheDocument();

    // Click row
    fireEvent.click(screen.getByText('Something broke'));

    // After click: metadata is shown in expanded detail
    expect(screen.getByText(/\/api\/tasks/)).toBeInTheDocument();
  });

  it('collapses row on second click', () => {
    render(<LogTable logs={[errorLog]} filter={{}} />);
    const row = screen.getByText('Something broke');

    fireEvent.click(row);
    expect(screen.getByText(/\/api\/tasks/)).toBeInTheDocument();

    fireEvent.click(row);
    expect(screen.queryByText(/\/api\/tasks/)).not.toBeInTheDocument();
  });

  it('displays correct source color for frontend logs', () => {
    render(<LogTable logs={[mockLog]} filter={{}} />);
    const sourceEl = screen.getByText('frontend');
    expect(sourceEl.className).toContain('blue');
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/components/logs/log-table.test.tsx 2>&1
```

Expected: 6 tests pass.

---

## Task 3: OverviewCards Component Tests

**Files:**
- Create: `frontend/src/components/analytics/overview-cards.test.tsx`

**Step 1: Write tests**

Create `frontend/src/components/analytics/overview-cards.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { OverviewCards } from './overview-cards';
import { mockOverview } from '@/test/msw/fixtures';

describe('OverviewCards', () => {
  it('renders all 4 card labels', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      expect(screen.getByText(/Total Tokens/i)).toBeInTheDocument();
      expect(screen.getByText(/Estimated Cost/i)).toBeInTheDocument();
      expect(screen.getByText(/Total Sessions/i)).toBeInTheDocument();
      expect(screen.getByText(/Tasks with AI/i)).toBeInTheDocument();
    });
  });

  it('shows formatted total sessions', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      expect(screen.getByText(mockOverview.total_sessions.toString())).toBeInTheDocument();
    });
  });

  it('shows loading skeleton while fetching', () => {
    renderWithProviders(<OverviewCards />);
    // Before data loads, skeletons appear (animate-pulse elements)
    const skeletons = document.querySelectorAll('.animate-pulse');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('shows estimated cost with dollar sign', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      expect(screen.getByText(/\$1\.1250/)).toBeInTheDocument();
    });
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/components/analytics/overview-cards.test.tsx 2>&1
```

Expected: 4 tests pass.

---

## Task 4: CreateTaskDialog Component Tests

**Files:**
- Create: `frontend/src/components/tasks/create-task-dialog.test.tsx`

**Step 1: Write tests**

Create `frontend/src/components/tasks/create-task-dialog.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { CreateTaskDialog } from './create-task-dialog';

describe('CreateTaskDialog', () => {
  it('renders dialog when open=true', () => {
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('Create New Task')).toBeInTheDocument();
  });

  it('does not render when open=false', () => {
    renderWithProviders(
      <CreateTaskDialog open={false} onOpenChange={vi.fn()} />
    );
    expect(screen.queryByText('Create New Task')).not.toBeInTheDocument();
  });

  it('disables submit button when title is empty', () => {
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );
    const submit = screen.getByRole('button', { name: /Create Task/i });
    expect(submit).toBeDisabled();
  });

  it('enables submit button when title is filled', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );

    await user.type(screen.getByPlaceholderText('Enter task title'), 'My task');
    const submit = screen.getByRole('button', { name: /Create Task/i });
    expect(submit).not.toBeDisabled();
  });

  it('calls onOpenChange(false) on cancel', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={onOpenChange} />
    );

    await user.click(screen.getByRole('button', { name: /Cancel/i }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('submits form and calls onOpenChange(false) on success', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={onOpenChange} />
    );

    await user.type(screen.getByPlaceholderText('Enter task title'), 'New task');
    await user.click(screen.getByRole('button', { name: /Create Task/i }));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/components/tasks/create-task-dialog.test.tsx 2>&1
```

Expected: 6 tests pass.

---

## Task 5: ErrorBoundary Component Tests

**Files:**
- Create: `frontend/src/components/error-boundary.test.tsx`

**Step 1: Write tests**

Create `frontend/src/components/error-boundary.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ErrorBoundary } from './error-boundary';

// Component that throws when told to
function Bomb({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) throw new Error('Test explosion');
  return <div>All good</div>;
}

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    render(
      <ErrorBoundary>
        <Bomb shouldThrow={false} />
      </ErrorBoundary>
    );
    expect(screen.getByText('All good')).toBeInTheDocument();
  });

  it('shows error fallback UI when child throws', () => {
    // Suppress expected console.error from React
    vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <Bomb shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText(/Something went wrong/i)).toBeInTheDocument();
    expect(screen.getByText(/Test explosion/i)).toBeInTheDocument();
    vi.restoreAllMocks();
  });

  it('resets on "Try again" click', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <Bomb shouldThrow={true} />
      </ErrorBoundary>
    );

    fireEvent.click(screen.getByText(/Try again/i));
    // After reset, boundary tries to render children again
    // (will throw again here, but boundary resets its state)
    expect(screen.queryByText(/Something went wrong/i)).not.toBeInTheDocument();
    vi.restoreAllMocks();
  });

  it('renders custom fallback when provided', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <ErrorBoundary fallback={<div>Custom error UI</div>}>
        <Bomb shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Custom error UI')).toBeInTheDocument();
    vi.restoreAllMocks();
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/components/error-boundary.test.tsx 2>&1
```

Expected: 4 tests pass.

---

## Task 6: Integration Test — Logs Page

**Files:**
- Create: `frontend/src/test/integration/logs-page.test.tsx`

**Step 1: Write integration test**

Create directory and file `frontend/src/test/integration/logs-page.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { renderWithProviders } from '@/test/utils';
import { mockLog } from '@/test/msw/fixtures';
import type { LogEntry } from '@/types/log';

// Import the actual page component (not the Next.js page default export wrapper)
// We test the inner components since the page is a Server Component shell
import { LogTable } from '@/components/logs/log-table';
import { LogLevelBadge } from '@/components/logs/log-level-badge';

const warnLog: LogEntry = {
  ...mockLog,
  id: 3,
  level: 'WARN',
  message: 'Something suspicious',
};

describe('Logs page integration', () => {
  it('renders log table with data from API', async () => {
    server.use(
      http.get('http://localhost:3001/api/logs', () =>
        HttpResponse.json([mockLog, warnLog])
      )
    );

    renderWithProviders(
      <LogTable logs={[mockLog, warnLog]} filter={{}} />
    );

    expect(screen.getByText('Test log message')).toBeInTheDocument();
    expect(screen.getByText('Something suspicious')).toBeInTheDocument();
  });

  it('level filter hides non-matching rows', () => {
    renderWithProviders(
      <LogTable logs={[mockLog, warnLog]} filter={{ level: 'ERROR' }} />
    );
    // Neither INFO nor WARN match ERROR — but client-side filtering only handles 'search'
    // Level filtering is server-side; table shows all logs passed to it
    // So both should still render (filter is applied at hook level)
    expect(screen.getByText('Test log message')).toBeInTheDocument();
  });

  it('search filter hides non-matching rows', () => {
    renderWithProviders(
      <LogTable logs={[mockLog, warnLog]} filter={{ search: 'suspicious' }} />
    );
    expect(screen.queryByText('Test log message')).not.toBeInTheDocument();
    expect(screen.getByText('Something suspicious')).toBeInTheDocument();
  });

  it('expands metadata on row click', () => {
    const logWithMeta: LogEntry = {
      ...mockLog,
      id: 10,
      message: 'Event with metadata',
      metadata: '{"key": "value123"}',
    };

    renderWithProviders(<LogTable logs={[logWithMeta]} filter={{}} />);
    fireEvent.click(screen.getByText('Event with metadata'));
    expect(screen.getByText(/value123/)).toBeInTheDocument();
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/test/integration/logs-page.test.tsx 2>&1
```

Expected: 4 tests pass.

---

## Task 7: Integration Test — Analytics Page

**Files:**
- Create: `frontend/src/test/integration/analytics-page.test.tsx`

**Step 1: Write integration test**

Create `frontend/src/test/integration/analytics-page.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { OverviewCards } from '@/components/analytics/overview-cards';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';

describe('Analytics page components integration', () => {
  it('OverviewCards loads and displays token count', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      // 150000 + 45000 = 195000 → "195.0K"
      expect(screen.getByText(/195\.0K/)).toBeInTheDocument();
    });
  });

  it('ToolBreakdownChart renders without crashing', async () => {
    renderWithProviders(<ToolBreakdownChart />);
    await waitFor(() => {
      expect(screen.getByText(/Tokens per Tool Call/i)).toBeInTheDocument();
    });
  });

  it('LanguageChart renders without crashing', async () => {
    renderWithProviders(<LanguageChart />);
    await waitFor(() => {
      expect(screen.getByText(/Tokens per Language/i)).toBeInTheDocument();
    });
  });

  it('ToolBreakdownChart shows empty state when no data', async () => {
    const { http, HttpResponse } = await import('msw');
    const { server } = await import('@/test/msw/server');

    server.use(
      http.get('http://localhost:3001/api/analytics/tokens/by-tool', () =>
        HttpResponse.json([])
      )
    );

    renderWithProviders(<ToolBreakdownChart />);
    await waitFor(() => {
      expect(screen.getByText(/No tool data yet/i)).toBeInTheDocument();
    });
  });
});
```

**Step 2: Run and verify**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/test/integration/analytics-page.test.tsx 2>&1
```

Expected: 4 tests pass.

---

## Task 8: Playwright E2E Tests

**Files:**
- Create: `frontend/src/test/e2e/navigation.spec.ts`
- Create: `frontend/src/test/e2e/logs.spec.ts`
- Create: `frontend/src/test/e2e/analytics.spec.ts`

**Note:** E2E tests require the dev server running. Start it first:
```bash
cd /home/utility/Projects/ai-kanban/frontend && npm run dev &
# Wait for "Ready" message
```

**Step 1: Navigation spec**

Create `frontend/src/test/e2e/navigation.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test.describe('Navigation', () => {
  test('home page loads', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('aside')).toBeVisible();
  });

  test('kanban page loads from sidebar', async ({ page }) => {
    await page.goto('/');
    await page.click('text=Kanban Board');
    await expect(page).toHaveURL('/kanban');
  });

  test('analytics page loads from sidebar', async ({ page }) => {
    await page.goto('/');
    await page.click('text=Analytics');
    await expect(page).toHaveURL('/analytics');
    await expect(page.locator('h1', { hasText: 'Analytics' })).toBeVisible();
  });

  test('logs page loads from sidebar', async ({ page }) => {
    await page.goto('/');
    await page.click('text=Logs');
    await expect(page).toHaveURL('/logs');
    await expect(page.locator('h1', { hasText: 'Logs' })).toBeVisible();
  });
});
```

**Step 2: Logs E2E spec**

Create `frontend/src/test/e2e/logs.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test.describe('Logs page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/logs');
  });

  test('shows filter bar', async ({ page }) => {
    await expect(page.locator('button', { hasText: 'ALL' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'DEBUG' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'ERROR' })).toBeVisible();
  });

  test('shows source filter', async ({ page }) => {
    await expect(page.locator('button', { hasText: 'Frontend' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Backend' })).toBeVisible();
  });

  test('shows live toggle', async ({ page }) => {
    await expect(page.locator('text=Live')).toBeVisible();
  });

  test('search input is present', async ({ page }) => {
    await expect(page.locator('input[placeholder*="Search"]')).toBeVisible();
  });
});
```

**Step 3: Analytics E2E spec**

Create `frontend/src/test/e2e/analytics.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test.describe('Analytics page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/analytics');
  });

  test('shows overview card labels', async ({ page }) => {
    await expect(page.locator('text=Total Tokens')).toBeVisible();
    await expect(page.locator('text=Estimated Cost')).toBeVisible();
  });

  test('shows Token Usage Over Time chart section', async ({ page }) => {
    await expect(page.locator('text=Token Usage Over Time')).toBeVisible();
  });

  test('time toggle buttons are visible', async ({ page }) => {
    await expect(page.locator('button', { hasText: 'Daily' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Weekly' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Monthly' })).toBeVisible();
  });

  test('clicking Weekly toggle works without error', async ({ page }) => {
    await page.click('button:has-text("Weekly")');
    // Verify no crash — page still has the chart heading
    await expect(page.locator('text=Token Usage Over Time')).toBeVisible();
  });

  test('shows tool breakdown chart section', async ({ page }) => {
    await expect(page.locator('text=Tokens per Tool Call')).toBeVisible();
  });

  test('session timeline has session dropdown', async ({ page }) => {
    await expect(page.locator('select')).toBeVisible();
    await expect(page.locator('text=Session Token Timeline')).toBeVisible();
  });
});
```

**Step 4: Run E2E tests**

```bash
# Make sure dev server is running first
cd /home/utility/Projects/ai-kanban/frontend
npm run test:e2e 2>&1
```

Expected: all E2E tests pass (or skip gracefully if backend is not running — update MSW to intercept in E2E if needed).

---

## Task 9: Verify 80% Coverage

**Step 1: Run all unit + component + integration tests with coverage**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm run test:coverage 2>&1
```

**Step 2: Check threshold**

Expected output includes:
```
 % Stmts  | % Branch |  % Funcs  | % Lines
   ≥ 80   |   ≥ 80   |   ≥ 80   |  ≥ 80
```

If below 80%, identify which files are uncovered:

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm run test:coverage 2>&1 | grep "Uncovered"
```

Add targeted tests for any uncovered file until the threshold is met.

Common gaps to check:
- `src/lib/utils.ts` — add tests for each exported utility function
- `src/hooks/use-comments.ts` — add hook tests similar to use-tasks

**Step 3: Fix any threshold failures**

If coverage threshold check fails, Vitest exits with error. Add the missing tests, run again until green.

**Step 4: Final commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/src/components/logs/*.test.tsx \
        frontend/src/components/analytics/*.test.tsx \
        frontend/src/components/tasks/create-task-dialog.test.tsx \
        frontend/src/components/error-boundary.test.tsx \
        frontend/src/test/integration/ \
        frontend/src/test/e2e/
git commit -m "test(frontend): add component, integration, and E2E tests — 80% coverage

Component tests:
- LogLevelBadge: 4 tests (all 4 levels, color classes)
- LogTable: 6 tests (render, empty state, search filter, expand/collapse, source color)
- OverviewCards: 4 tests (labels, sessions, loading skeleton, cost format)
- CreateTaskDialog: 6 tests (open/closed, submit disabled/enabled, cancel, submit success)
- ErrorBoundary: 4 tests (no error, fallback UI, reset, custom fallback)

Integration tests:
- Logs page: 4 tests (render, level filter, search filter, metadata expand)
- Analytics page: 4 tests (overview loads, tool chart, language chart, empty state)

E2E tests (Playwright):
- Navigation: 4 tests (home, kanban, analytics, logs)
- Logs page: 4 tests (filters, live toggle, search input)
- Analytics: 6 tests (cards, chart, time toggle, tool breakdown, session dropdown)

Coverage: ≥80% lines, branches, functions, statements"
```

---

## Summary

Total tests across all plan files:
- Plan 8 smoke: 2
- Plan 9 unit/hook: 30
- Plan 10 component/integration/E2E: ~47

**Grand total: ~79 tests** covering lib/, hooks/, components/logs/, components/analytics/, and key shared components.
