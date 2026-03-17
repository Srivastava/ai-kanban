# Analytics Chart Improvements Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace single-bar "Lines Written" and "Token Usage" charts with meaningful multi-point visualizations, convert Token Efficiency to a scatter plot with task filter support, and add a Cumulative Cost curve.

**Architecture:** Four independent changes — two backend additions (efficiency task filter, LOC history endpoint), two frontend chart rebuilds (DevActivityCharts, TokenEfficiencyChart), and one new chart (CumulativeCostChart). Each task can be built and committed independently.

**Tech Stack:** Rust/Axum backend, SQLite/sqlx, Next.js frontend, Recharts, React Query, TypeScript

---

## File Map

| File | Change |
|---|---|
| `backend/src/db/analytics.rs` | Add `task_id` param to `token_efficiency()`, add `session_loc_history()` |
| `backend/src/api/analytics.rs` | Wire task filter to efficiency handler, add `/tasks/:id/loc-history` route |
| `frontend/src/types/analytics.ts` | Add `LocHistoryEntry` type |
| `frontend/src/hooks/use-analytics.ts` | Add `taskId` to `useTokenEfficiency`, add `useLocHistory` hook |
| `frontend/src/components/analytics/token-efficiency-chart.tsx` | Rebuild as scatter plot (global) + stat card (per-task), accept `taskId` prop |
| `frontend/src/components/analytics/dev-activity-charts.tsx` | Replace LOC bar → line chart; replace token bar → stacked area chart |
| `frontend/src/components/analytics/cumulative-cost-chart.tsx` | New: running cost area chart, respects `taskId` |
| `frontend/src/components/analytics/productivity-section.tsx` | Pass `taskId` to `<TokenEfficiencyChart taskId={taskId} />` (line 61) |
| `frontend/src/components/analytics/analytics-page-inner.tsx` | Add `CumulativeCostChart` to ROI & Cost section |

---

## Task 1: Backend — token_efficiency task filter

**Files:**
- Modify: `backend/src/db/analytics.rs` (token_efficiency function, ~line 421)
- Modify: `backend/src/api/analytics.rs` (efficiency handler, ~line 180)

- [ ] **Step 1: Add task_id param to token_efficiency in db/analytics.rs**

Find `pub async fn token_efficiency(&self) -> Result<Vec<EfficiencyRow>>` and change to:

```rust
pub async fn token_efficiency(&self, task_id: Option<&str>) -> Result<Vec<EfficiencyRow>> {
```

Add task filter logic before the query (same pattern as `daily_tokens`):
```rust
let task_filter = if task_id.is_some() { " AND te.task_id = ?" } else { "" };
let sql = format!(
    r#"
    SELECT
        te.task_id,
        COALESCE(t.title, 'Unknown Task') as task_title,
        SUM(te.input_tokens) + SUM(te.output_tokens) as total_tokens,
        CAST(
            COALESCE(MAX(sm.project_loc), 0)
            - COALESCE(MIN(CASE WHEN sm.project_loc > 0 THEN sm.project_loc END), 0)
        AS REAL) as lines_written,
        CAST(COALESCE(MAX(sm.project_loc), 0) AS INTEGER) as project_loc
    FROM token_events te
    LEFT JOIN tasks t ON te.task_id = t.id
    LEFT JOIN session_metrics sm ON te.session_id = sm.session_id
    WHERE te.event_type = 'assistant' AND te.task_id IS NOT NULL{task_filter}
    GROUP BY te.task_id, t.title
    HAVING total_tokens > 0
    ORDER BY total_tokens DESC
    "#,
    task_filter = task_filter
);
let rows = if let Some(tid) = task_id {
    sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
} else {
    sqlx::query(&sql).fetch_all(&self.pool).await?
};
```

- [ ] **Step 2: Update efficiency handler in api/analytics.rs**

Find the efficiency handler (currently calls `state.analytics.token_efficiency()`) and update:

```rust
pub async fn token_efficiency(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<TaskFilterQuery>,
) -> impl IntoResponse {
    match state.analytics.token_efficiency(query.task_id.as_deref()).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            tracing::error!("token_efficiency error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

(`TaskFilterQuery` struct already exists in api/analytics.rs)

- [ ] **Step 3: Build backend**

```bash
cd backend && cargo build
```
Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add backend/src/db/analytics.rs backend/src/api/analytics.rs
git commit -m "feat: add task_id filter to token_efficiency endpoint"
```

---

## Task 2: Backend — per-session LOC history endpoint

**Files:**
- Modify: `backend/src/db/analytics.rs` (add `session_loc_history`)
- Modify: `backend/src/api/analytics.rs` (add route + handler)
- Modify: `backend/src/models/analytics.rs` (add `LocHistoryEntry` struct)

- [ ] **Step 1: Add LocHistoryEntry model**

In `backend/src/models/analytics.rs`, add:

```rust
#[derive(Debug, serde::Serialize)]
pub struct LocHistoryEntry {
    pub session_id: String,
    pub session_index: i64,
    pub project_loc: i64,
    pub started_at: String,
}
```

(`backend/src/models/mod.rs` already has `pub use analytics::*` glob — no change needed there.)

- [ ] **Step 2: Add session_loc_history to AnalyticsRepository**

In `backend/src/db/analytics.rs`, add at the end of `impl AnalyticsRepository`:

```rust
pub async fn session_loc_history(&self, task_id: &str) -> Result<Vec<crate::models::LocHistoryEntry>> {
    let rows = sqlx::query(
        r#"
        SELECT
            sm.session_id,
            sm.project_loc,
            s.started_at,
            ROW_NUMBER() OVER (ORDER BY s.started_at ASC) as session_index
        FROM session_metrics sm
        JOIN sessions s ON s.id = sm.session_id
        WHERE s.task_id = ? AND sm.project_loc > 0
        ORDER BY s.started_at ASC
        "#
    )
    .bind(task_id)
    .fetch_all(&self.pool)
    .await?;

    Ok(rows.into_iter().map(|row| crate::models::LocHistoryEntry {
        session_id: row.get("session_id"),
        session_index: row.get("session_index"),
        project_loc: row.get("project_loc"),
        started_at: row.get("started_at"),
    }).collect())
}
```

- [ ] **Step 3: Add route and handler in api/analytics.rs**

Add to `analytics_routes()`:
```rust
.route("/tasks/:task_id/loc-history", get(loc_history))
```

Add handler:
```rust
pub async fn loc_history(
    State(state): State<AnalyticsApiState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    match state.analytics.session_loc_history(&task_id).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            tracing::error!("loc_history error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

- [ ] **Step 4: Build backend**

```bash
cd backend && cargo build
```
Expected: compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add backend/src/db/analytics.rs backend/src/api/analytics.rs backend/src/models/analytics.rs backend/src/models/mod.rs
git commit -m "feat: add session LOC history endpoint for per-task LOC growth chart"
```

---

## Task 3: Frontend hooks — useTokenEfficiency taskId + useLocHistory

**Files:**
- Modify: `frontend/src/types/analytics.ts`
- Modify: `frontend/src/hooks/use-analytics.ts`

- [ ] **Step 1: Add LocHistoryEntry type**

In `frontend/src/types/analytics.ts`, add:

```typescript
export interface LocHistoryEntry {
  session_id: string;
  session_index: number;
  project_loc: number;
  started_at: string;
}
```

- [ ] **Step 2: Update useTokenEfficiency to accept taskId**

In `frontend/src/hooks/use-analytics.ts`, replace:

```typescript
export function useTokenEfficiency() {
  ...
  return useQuery({
    queryKey: ['analytics', 'efficiency'],
    queryFn: async () => {
      ...
      const result = await apiClient<EfficiencyRow[]>('/api/analytics/tokens/efficiency');
```

With:

```typescript
export function useTokenEfficiency(taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'efficiency', taskId],
    queryFn: async () => {
      const url = taskId
        ? `/api/analytics/tokens/efficiency?task_id=${taskId}`
        : '/api/analytics/tokens/efficiency';
      const result = await apiClient<EfficiencyRow[]>(url);
      logger.debug('useTokenEfficiency: fetch complete', { count: result.length });
      return result;
    },
  });
}
```

- [ ] **Step 3: Add useLocHistory hook**

Add after `useTokenEfficiency` in `use-analytics.ts`:

```typescript
export function useLocHistory(taskId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'loc-history', taskId],
    queryFn: () => apiClient<LocHistoryEntry[]>(`/api/analytics/tasks/${taskId}/loc-history`),
    enabled: !!taskId,
  });
}
```

Also add `LocHistoryEntry` to the import at the top of `use-analytics.ts`.

- [ ] **Step 4: TypeScript check**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/types/analytics.ts frontend/src/hooks/use-analytics.ts
git commit -m "feat: add taskId to useTokenEfficiency, add useLocHistory hook"
```

---

## Task 4: Frontend — rebuild TokenEfficiencyChart

**Files:**
- Modify: `frontend/src/components/analytics/token-efficiency-chart.tsx`

**Design:**
- **Global view** (`taskId` is null): scatter plot. X = lines written, Y = tokens used. Each dot is a task, labeled. Tasks near bottom-right are most efficient.
- **Per-task view** (`taskId` set): stat card showing tokens/line for that task + its rank among all tasks ("2nd most efficient of 4 tasks").

- [ ] **Step 1: Rewrite token-efficiency-chart.tsx**

```tsx
'use client';

import { ScatterChart, Scatter, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Label } from 'recharts';
import { useTokenEfficiency } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return `${n}`;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

interface Props { taskId?: string | null }

export function TokenEfficiencyChart({ taskId }: Props) {
  const { data: allData = [], isLoading } = useTokenEfficiency();

  const chartData = allData
    .filter((d) => d.lines_written > 0 && d.total_tokens > 0)
    .map((d) => ({
      task_id: d.task_id,
      task_title: d.task_title,
      lines: Math.round(d.lines_written),
      tokens: d.total_tokens,
      tpl: d.tokens_per_line ? Math.round(d.tokens_per_line) : Math.round(d.total_tokens / d.lines_written),
      isSelected: d.task_id === taskId,
    }));

  // Per-task stat card
  if (taskId) {
    const row = chartData.find((d) => d.task_id === taskId) ?? allData.find((d) => d.task_id === taskId);
    const allWithLoc = allData.filter((d) => d.lines_written > 0);
    const sorted = [...allWithLoc].sort((a, b) => (a.tokens_per_line ?? 0) - (b.tokens_per_line ?? 0));
    const rank = row ? sorted.findIndex((d) => d.task_id === taskId) + 1 : null;
    const tpl = row && 'tokens_per_line' in row && row.tokens_per_line
      ? Math.round(row.tokens_per_line)
      : row && 'tpl' in row ? (row as typeof chartData[0]).tpl : null;

    return (
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <div>
          <h3 className="font-semibold">Token Efficiency</h3>
          <p className="text-xs text-muted-foreground mt-0.5">Tokens per line of code written — lower is more efficient</p>
        </div>
        {isLoading ? (
          <div className="h-24 animate-pulse bg-muted rounded" />
        ) : tpl == null ? (
          <div className="h-24 flex items-center justify-center">
            <p className="text-muted-foreground text-sm">No LOC data for this task</p>
          </div>
        ) : (
          <div className="flex items-end gap-6">
            <div>
              <p className="text-4xl font-bold tabular-nums text-indigo-500">{tpl}</p>
              <p className="text-xs text-muted-foreground mt-1">tokens / line</p>
            </div>
            {rank != null && allWithLoc.length > 1 && (
              <div className="pb-1">
                <p className="text-sm text-muted-foreground">
                  #{rank} most efficient of {allWithLoc.length} tasks
                </p>
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  // Global scatter plot
  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div>
        <h3 className="font-semibold">Token Efficiency</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Each dot is a task — bottom-right is most efficient (more code, fewer tokens)
        </p>
      </div>
      {isLoading ? (
        <div className="h-64 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-64 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No efficiency data yet — run sessions with LOC tracking enabled</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={280}>
          <ScatterChart margin={{ top: 10, right: 20, left: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis
              dataKey="lines"
              type="number"
              tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : `${v}`}
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
            >
              <Label value="Lines written" position="insideBottom" offset={-15} style={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
            </XAxis>
            <YAxis
              dataKey="tokens"
              type="number"
              tickFormatter={formatTokens}
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
            >
              <Label value="Total tokens" angle={-90} position="insideLeft" offset={15} style={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
            </YAxis>
            <Tooltip
              cursor={{ strokeDasharray: '3 3' }}
              content={({ active, payload }) => {
                if (!active || !payload?.length) return null;
                const d = payload[0].payload;
                return (
                  <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1 shadow-sm">
                    <p className="font-medium">{d.task_title}</p>
                    <p>{formatTokens(d.tokens)} tokens · {d.lines.toLocaleString()} lines</p>
                    <p className="text-indigo-400 font-medium">{d.tpl} tokens/line</p>
                  </div>
                );
              }}
              contentStyle={TOOLTIP_STYLE}
            />
            <Scatter
              data={chartData}
              fill="#6366f1"
              fillOpacity={0.8}
            />
          </ScatterChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/analytics/token-efficiency-chart.tsx
git commit -m "feat: rebuild TokenEfficiencyChart as scatter plot (global) + stat card (per-task)"
```

---

## Task 5: Frontend — rebuild DevActivityCharts

**Files:**
- Modify: `frontend/src/components/analytics/dev-activity-charts.tsx`

**Design:**
- Replace LOC single bar → **line chart** of `project_loc` across sessions (from `useLocHistory`)
- Replace token usage single bar → **stacked area chart** of input/output/cache across sessions (from `useTaskSessions`)
- Keep the summary stats row (sessions, lines, tokens, cost)

- [ ] **Step 1: Rewrite dev-activity-charts.tsx**

```tsx
'use client';

import {
  LineChart, Line, AreaChart, Area,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend,
} from 'recharts';
import { useDevActivity, useLocHistory, useTaskSessions, useTokensByTask } from '@/hooks/use-analytics';
import { useState } from 'react';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return `${n}`;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

interface Props { taskId?: string | null }

export function DevActivityCharts({ taskId: externalTaskId }: Props) {
  const [internalTaskId, setInternalTaskId] = useState<string | null>(null);
  const selectedTaskId = externalTaskId ?? internalTaskId;
  const { data: tasks = [] } = useTokensByTask();
  const { data = [], isLoading } = useDevActivity(selectedTaskId);
  const { data: locHistory = [] } = useLocHistory(selectedTaskId);
  const { data: sessions = [] } = useTaskSessions(selectedTaskId);

  const row = data[0] ?? null;

  // LOC growth line chart data
  const locData = locHistory.map((entry) => ({
    label: `#${entry.session_index}`,
    loc: entry.project_loc,
  }));

  // Token stacked area chart data — sessions ordered chronologically.
  // Note: SessionDetail does not include cache_creation/cache_read tokens,
  // so we show only input + output (two-layer stack).
  const sessionTokenData = [...sessions]
    .reverse() // sessions come newest-first, reverse for timeline
    .map((s, i) => ({
      label: `#${i + 1}`,
      input: s.input_tokens,
      output: s.output_tokens,
    }));

  const skeleton = <div className="h-40 animate-pulse bg-muted rounded" />;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      {/* Task selector — hidden when external taskId provided */}
      {!externalTaskId && (
        <select
          className="rounded-md border border-border bg-background px-3 py-1.5 text-sm max-w-xs"
          value={internalTaskId ?? ''}
          onChange={(e) => setInternalTaskId(e.target.value || null)}
        >
          <option value="">Select a task…</option>
          {tasks.map((t) => (
            <option key={t.task_id} value={t.task_id}>{t.task_title}</option>
          ))}
        </select>
      )}

      {!selectedTaskId ? (
        <div className="h-40 flex items-center justify-center rounded-lg border border-dashed border-border">
          <p className="text-muted-foreground text-sm">Select a task to view dev activity</p>
        </div>
      ) : isLoading ? (
        skeleton
      ) : !row ? (
        <div className="h-40 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No data for this task</p>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Summary row */}
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Sessions</p>
              <p className="font-semibold">{row.session_count}</p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Lines written (net)</p>
              <p className="font-semibold">
                {row.lines_added > 0
                  ? <span className="text-green-600">+{Math.round(row.lines_added).toLocaleString()}</span>
                  : '—'}
              </p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Tokens (in / cached / out)</p>
              <p className="font-semibold text-sm">
                <span className="text-blue-500">{formatTokens(row.input_tokens ?? 0)}</span>
                {' / '}
                <span className="text-amber-500">{formatTokens((row.cache_creation_tokens ?? 0) + (row.cache_read_tokens ?? 0))}</span>
                {' / '}
                <span className="text-violet-500">{formatTokens(row.output_tokens ?? 0)}</span>
              </p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Cost (OTel)</p>
              <p className="font-semibold">{row.cost_usd > 0 ? `$${row.cost_usd.toFixed(4)}` : '—'}</p>
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            {/* LOC growth line chart */}
            <div className="rounded-xl border border-border bg-card p-5 space-y-2">
              <h3 className="font-semibold text-sm">Project LOC Over Sessions</h3>
              <p className="text-xs text-muted-foreground">How the codebase grew session by session</p>
              {locData.length < 2 ? (
                <div className="h-40 flex items-center justify-center">
                  <p className="text-xs text-muted-foreground italic">
                    {locData.length === 0 ? 'No LOC snapshots yet' : 'Only one session — need more to show trend'}
                  </p>
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={160}>
                  <LineChart data={locData} margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <YAxis
                      tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : `${v}`}
                      tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                    />
                    <Tooltip
                      formatter={(value) => [typeof value === 'number' ? value.toLocaleString() : value, 'Lines of code']}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Line
                      type="monotone"
                      dataKey="loc"
                      stroke="#22c55e"
                      strokeWidth={2}
                      dot={{ fill: '#22c55e', r: 3 }}
                      activeDot={{ r: 5 }}
                    />
                  </LineChart>
                </ResponsiveContainer>
              )}
            </div>

            {/* Token stacked area chart */}
            <div className="rounded-xl border border-border bg-card p-5 space-y-2">
              <h3 className="font-semibold text-sm">Token Usage Per Session</h3>
              <p className="text-xs text-muted-foreground">Input and output tokens per session</p>
              {sessionTokenData.length < 2 ? (
                <div className="h-40 flex items-center justify-center">
                  <p className="text-xs text-muted-foreground italic">
                    {sessionTokenData.length === 0 ? 'No session data' : 'Only one session — need more to show trend'}
                  </p>
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={160}>
                  <AreaChart data={sessionTokenData} margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <Tooltip
                      formatter={(value, name) => {
                        const labels: Record<string, string> = { input: 'Input', output: 'Output' };
                        return [formatTokens(Number(value)), labels[String(name)] ?? String(name)];
                      }}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Legend formatter={(v) => ({ input: 'Input', output: 'Output' }[v] ?? v)} iconType="circle" />
                    <Area type="monotone" dataKey="input"  stackId="1" stroke="#6366f1" fill="#6366f1" fillOpacity={0.6} />
                    <Area type="monotone" dataKey="output" stackId="1" stroke="#a855f7" fill="#a855f7" fillOpacity={0.6} />
                  </AreaChart>
                </ResponsiveContainer>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/analytics/dev-activity-charts.tsx
git commit -m "feat: rebuild DevActivityCharts with LOC line chart and token stacked area chart"
```

---

## Task 6: Frontend — new CumulativeCostChart

**Files:**
- Create: `frontend/src/components/analytics/cumulative-cost-chart.tsx`
- Modify: `frontend/src/components/analytics/analytics-page-inner.tsx`

**Design:** Area chart of running cumulative cost over time. Uses `useDailyTokens` (which already supports `taskId`). Computes running total client-side from daily token counts using the pricing constants.

- [ ] **Step 1: Create cumulative-cost-chart.tsx**

```tsx
'use client';

import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useDailyTokens } from '@/hooks/use-analytics';

// Pricing (must match backend token_prices defaults)
const PRICE = {
  input: 3.0 / 1_000_000,
  output: 15.0 / 1_000_000,
  cacheWrite: 3.75 / 1_000_000,
  cacheRead: 0.30 / 1_000_000,
};

function dayCost(d: { input_tokens: number; output_tokens: number; cache_creation_tokens: number; cache_read_tokens: number }): number {
  return d.input_tokens * PRICE.input
    + d.output_tokens * PRICE.output
    + d.cache_creation_tokens * PRICE.cacheWrite
    + d.cache_read_tokens * PRICE.cacheRead;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

interface Props { taskId?: string | null }

export function CumulativeCostChart({ taskId }: Props) {
  const { data: daily = [], isLoading } = useDailyTokens(30, taskId);

  let running = 0;
  const chartData = daily.map((d) => {
    running += dayCost(d);
    return {
      date: d.date.slice(5), // strip year: "03-17"
      cumulative: parseFloat(running.toFixed(4)),
      daily: parseFloat(dayCost(d).toFixed(4)),
    };
  });

  const totalCost = running;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="font-semibold">Cumulative Cost (30 days)</h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {taskId ? 'Running spend for selected task' : 'Running spend across all tasks'}
          </p>
        </div>
        {!isLoading && totalCost > 0 && (
          <span className="text-lg font-bold tabular-nums text-emerald-500">
            ${totalCost.toFixed(4)}
          </span>
        )}
      </div>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No cost data in the last 30 days</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={200}>
          <AreaChart data={chartData} margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
            <defs>
              <linearGradient id="costGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#10b981" stopOpacity={0.02} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
            <XAxis dataKey="date" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
            <YAxis
              tickFormatter={(v) => `$${v.toFixed(2)}`}
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
            />
            <Tooltip
              formatter={(value, name) => [
                `$${Number(value).toFixed(4)}`,
                name === 'cumulative' ? 'Cumulative' : 'Daily',
              ]}
              contentStyle={TOOLTIP_STYLE}
            />
            <Area
              type="monotone"
              dataKey="cumulative"
              stroke="#10b981"
              strokeWidth={2}
              fill="url(#costGradient)"
            />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Pass taskId to TokenEfficiencyChart in productivity-section.tsx**

`TokenEfficiencyChart` is rendered at line 61 of `productivity-section.tsx`. Change:
```tsx
<TokenEfficiencyChart />
```
to:
```tsx
<TokenEfficiencyChart taskId={taskId} />
```
(`ProductivitySection` already receives `taskId` as a prop.)

- [ ] **Step 3: Add CumulativeCostChart to analytics-page-inner.tsx**

In `analytics-page-inner.tsx`, add import:
```tsx
import { CumulativeCostChart } from '@/components/analytics/cumulative-cost-chart';
```

In the ROI & Cost section, after the `<div className="grid ...">` containing `CostBreakdownTable` and `TokensByTaskChart`, add:
```tsx
<CumulativeCostChart taskId={selectedTaskId} />
```

- [ ] **Step 4: TypeScript check**

```bash
cd frontend && npx tsc --noEmit
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/analytics/cumulative-cost-chart.tsx frontend/src/components/analytics/analytics-page-inner.tsx frontend/src/components/analytics/productivity-section.tsx
git commit -m "feat: add CumulativeCostChart, wire taskId to TokenEfficiencyChart"
```

---

## Task 7: Final integration check

- [ ] **Step 1: Start the dev server and visually verify**

```bash
cd frontend && npm run dev
```

Open `http://localhost:3000/analytics` and verify:
1. **Global view (no task filter):** TokenEfficiencyChart shows scatter plot with task dots
2. **Select a task:** TokenEfficiencyChart shows stat card with rank
3. **Select a task:** DevActivityCharts shows LOC line chart (if task has ≥2 sessions with LOC data)
4. **Select a task:** DevActivityCharts shows token stacked area chart (if task has ≥2 sessions)
5. **Global and per-task:** CumulativeCostChart shows running cost area chart

- [ ] **Step 2: Final commit**

```bash
git add -A
git commit -m "feat: analytics chart improvements — scatter plot efficiency, LOC growth, token area, cumulative cost"
```
