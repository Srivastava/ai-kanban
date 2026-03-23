# Analytics v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real-time live session updates, an activity heatmap, hour-of-day chart, project bubble chart, and session tool drill-down to the analytics tab — all strictly additive.

**Architecture:** Three independent layers built bottom-up: (1) backend DB methods + endpoints, (2) frontend types/hooks/shared utilities, (3) UI components wired into the existing `AnalyticsPageInner` layout. Existing chart components are never modified. Auto-refresh uses a single WS subscription at the page level.

**Tech Stack:** Rust/Axum/sqlx (backend), Next.js/React Query/Recharts/SVG (frontend), existing WebSocket context, existing `AnalyticsApiState`.

**Key constraint:** Rate limits on `/usage` — polling intervals must be conservative. Live cost reuses the existing `useAnalyticsOverview` cache (already polls every 30s); we only trigger faster refetch when a session is actively running (10s max, paused otherwise).

**Spec:** `docs/superpowers/specs/2026-03-21-analytics-v2-design.md`

---

## File Map

**New backend files:** none — all changes in existing files

**Modified backend:**
- `backend/src/db/analytics.rs` — add `daily_heatmap()`, `hourly_breakdown()`, `tokens_by_tool_for_session()`
- `backend/src/api/analytics.rs` — add 3 new route handlers + register routes

**New frontend files:**
- `frontend/src/lib/pricing.ts` — shared pricing constants + `estimateCost()`
- `frontend/src/components/analytics/activity-heatmap.tsx` — SVG heatmap
- `frontend/src/components/analytics/hourly-breakdown.tsx` — Recharts bar chart
- `frontend/src/components/analytics/project-bubble-chart.tsx` — SVG bubble chart
- `frontend/src/components/analytics/session-tool-breakdown.tsx` — inline drill-down

**Modified frontend:**
- `frontend/src/types/analytics.ts` — add `HeatmapEntry`, `HourlyEntry`, `SessionToolTokens` interfaces
- `frontend/src/hooks/use-analytics.ts` — add `useDailyHeatmap`, `useHourlyBreakdown`, `useSessionTools`
- `frontend/src/components/analytics/command-center.tsx` — live banner + polling ticker + sparkline
- `frontend/src/components/analytics/analytics-page-inner.tsx` — WS auto-refresh + new sections
- `frontend/src/components/analytics/session-timeline-chart.tsx` — click handler → drill-down

---

## Task 1: Backend — New DB Methods

**Files:**
- Modify: `backend/src/db/analytics.rs`

- [ ] **Step 1: Add `daily_heatmap` DB method**

Add to `AnalyticsRepository` in `backend/src/db/analytics.rs`, after the existing `tokens_by_language` method:

```rust
/// Token activity per calendar day (UTC) for heatmap display.
pub async fn daily_heatmap(
    &self,
    days: i64,
    task_id: Option<&str>,
) -> Result<Vec<HeatmapEntry>> {
    let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
    let sql = format!(
        r#"
        SELECT
            DATE(created_at) as date,
            SUM(input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens) as tokens
        FROM token_events
        WHERE DATE(created_at) >= DATE('now', '-{days} days'){task_filter}
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#,
        days = days,
        task_filter = task_filter
    );
    let rows = if let Some(tid) = task_id {
        sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
    } else {
        sqlx::query(&sql).fetch_all(&self.pool).await?
    };
    Ok(rows
        .into_iter()
        .map(|row| HeatmapEntry {
            date: row.get("date"),
            tokens: row.get("tokens"),
        })
        .collect())
}

/// Token activity per hour of day (0–23 UTC).
pub async fn hourly_breakdown(
    &self,
    task_id: Option<&str>,
) -> Result<Vec<HourlyEntry>> {
    let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
    let sql = format!(
        r#"
        SELECT
            CAST(strftime('%H', created_at) AS INTEGER) as hour,
            SUM(input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens) as tokens
        FROM token_events
        WHERE 1=1{task_filter}
        GROUP BY hour
        ORDER BY hour ASC
        "#,
        task_filter = task_filter
    );
    let rows = if let Some(tid) = task_id {
        sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
    } else {
        sqlx::query(&sql).fetch_all(&self.pool).await?
    };
    Ok(rows
        .into_iter()
        .map(|row| HourlyEntry {
            hour: row.get("hour"),
            tokens: row.get("tokens"),
        })
        .collect())
}

/// Tool breakdown for a specific session (for drill-down).
pub async fn tokens_by_tool_for_session(
    &self,
    session_id: &str,
) -> Result<Vec<ToolTokens>> {
    let rows = sqlx::query(
        r#"
        SELECT
            COALESCE(tool_name, 'Other') as tool_name,
            SUM(input_tokens) as input_tokens,
            SUM(output_tokens) as output_tokens,
            COUNT(*) as call_count
        FROM token_events
        WHERE session_id = ?
        GROUP BY COALESCE(tool_name, 'Other')
        ORDER BY (SUM(input_tokens) + SUM(output_tokens)) DESC
        "#,
    )
    .bind(session_id)
    .fetch_all(&self.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ToolTokens {
            tool_name: row.get("tool_name"),
            input_tokens: row.get("input_tokens"),
            output_tokens: row.get("output_tokens"),
            call_count: row.get("call_count"),
        })
        .collect())
}
```

- [ ] **Step 2: Add structs `HeatmapEntry` and `HourlyEntry` to models**

All result structs (`ToolTokens`, `TaskTokens`, etc.) live in `backend/src/models/analytics.rs` — follow the same pattern. Add there:

```rust
#[derive(Debug, serde::Serialize)]
pub struct HeatmapEntry {
    pub date: String,
    pub tokens: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct HourlyEntry {
    pub hour: i64,
    pub tokens: i64,
}
```

Then add them to the `use crate::models::...` import at the top of `backend/src/db/analytics.rs` alongside the existing model imports.

- [ ] **Step 3: Export new types from `backend/src/models/mod.rs`**

Find the existing public re-exports in `backend/src/models/mod.rs` (or `backend/src/models/analytics.rs` pub usage) and ensure `HeatmapEntry` and `HourlyEntry` are publicly accessible via `crate::models`. Check the existing export pattern and follow it exactly.

> **Note:** `ToolTokens` is already exported from `crate::models` — `tokens_by_tool_for_session` returns `Vec<ToolTokens>` using the existing type with no new import needed in the handler.

- [ ] **Step 4: Build to verify**

```bash
cd backend && cargo build 2>&1 | grep -E "^error"
```
Expected: no output (clean build).

- [ ] **Step 5: Commit**

```bash
git add backend/src/db/analytics.rs backend/src/db/mod.rs
git commit -m "feat(backend): add daily_heatmap, hourly_breakdown, tokens_by_tool_for_session DB methods"
```

---

## Task 2: Backend — New API Endpoints

**Files:**
- Modify: `backend/src/api/analytics.rs`

- [ ] **Step 1: Add query structs**

In `backend/src/api/analytics.rs`, add after the existing `TaskFilterQuery` struct:

```rust
#[derive(Deserialize, Debug)]
struct HeatmapQuery {
    #[serde(default = "default_heatmap_days")]
    days: i64,
    task_id: Option<String>,
}

fn default_heatmap_days() -> i64 { 365 }
```

- [ ] **Step 2: Add handler functions**

Add before the closing of the file (before or after existing handlers):

```rust
#[instrument(skip(state))]
async fn daily_heatmap(
    State(state): State<AnalyticsApiState>,
    Query(q): Query<HeatmapQuery>,
) -> impl IntoResponse {
    match state.analytics.daily_heatmap(q.days, q.task_id.as_deref()).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "daily_heatmap failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn hourly_breakdown(
    State(state): State<AnalyticsApiState>,
    Query(q): Query<TaskFilterQuery>,
) -> impl IntoResponse {
    match state.analytics.hourly_breakdown(q.task_id.as_deref()).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "hourly_breakdown failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn session_tools(
    State(state): State<AnalyticsApiState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.analytics.tokens_by_tool_for_session(&session_id).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(session_id = %session_id, error = %e, "session_tools failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}
```

- [ ] **Step 3: Register routes**

In `analytics_routes()`, add three new routes. The `/sessions/:id/tools` route must be added **before** the existing `/sessions/:id/timeline` to avoid Axum path conflicts (though Axum handles this fine since the suffix differs):

```rust
.route("/daily-heatmap", get(daily_heatmap))
.route("/hourly-breakdown", get(hourly_breakdown))
.route("/sessions/:id/tools", get(session_tools))
```

- [ ] **Step 4: Build and run backend tests**

```bash
cd backend && cargo build 2>&1 | grep "^error" && cargo test 2>&1 | grep -E "FAILED|^test result"
```
Expected: clean build, all tests pass.

- [ ] **Step 5: Smoke test endpoints** (requires running backend)

```bash
curl -s http://localhost:3001/api/analytics/daily-heatmap?days=7 | python3 -m json.tool | head -10
curl -s http://localhost:3001/api/analytics/hourly-breakdown | python3 -m json.tool | head -10
```
Expected: JSON arrays (may be empty if no data).

- [ ] **Step 6: Commit**

```bash
git add backend/src/api/analytics.rs
git commit -m "feat(backend): add daily-heatmap, hourly-breakdown, sessions/:id/tools endpoints"
```

---

## Task 3: Frontend — Types, Shared Pricing, New Hooks

**Files:**
- Modify: `frontend/src/types/analytics.ts`
- Create: `frontend/src/lib/pricing.ts`
- Modify: `frontend/src/hooks/use-analytics.ts`
- Modify: `frontend/src/components/analytics/command-center.tsx` (pricing import only)

- [ ] **Step 1: Add new types to `analytics.ts`**

Append to `frontend/src/types/analytics.ts`:

```ts
export interface HeatmapEntry {
  date: string;   // "YYYY-MM-DD"
  tokens: number;
}

export interface HourlyEntry {
  hour: number;   // 0–23
  tokens: number;
}

export interface SessionToolTokens {
  tool_name: string;
  input_tokens: number;
  output_tokens: number;
  call_count: number;
}
```

- [ ] **Step 2: Create `frontend/src/lib/pricing.ts`**

```ts
// Single source of truth for Claude token pricing.
// All analytics components must import from here — never define prices inline.
export const PRICING = {
  input:      3.0,   // USD per 1M tokens
  output:     15.0,
  cacheWrite: 3.75,
  cacheRead:  0.30,
} as const;

export function estimateCost(
  input: number,
  output: number,
  cacheWrite = 0,
  cacheRead = 0,
): number {
  return (
    (input      / 1_000_000) * PRICING.input +
    (output     / 1_000_000) * PRICING.output +
    (cacheWrite / 1_000_000) * PRICING.cacheWrite +
    (cacheRead  / 1_000_000) * PRICING.cacheRead
  );
}
```

- [ ] **Step 3: Update `command-center.tsx` to use shared pricing**

In `frontend/src/components/analytics/command-center.tsx`, replace the four inline price constants:
```ts
// REMOVE these:
const INPUT_PRICE = 3.0;
const OUTPUT_PRICE = 15.0;
const CACHE_WRITE_PRICE = 3.75;
const CACHE_READ_PRICE = 0.30;
```

Add import:
```ts
import { PRICING } from '@/lib/pricing';
```

Replace all usages: `INPUT_PRICE` → `PRICING.input`, `OUTPUT_PRICE` → `PRICING.output`, `CACHE_WRITE_PRICE` → `PRICING.cacheWrite`, `CACHE_READ_PRICE` → `PRICING.cacheRead`.

- [ ] **Step 4: Add new hooks to `use-analytics.ts`**

Append to `frontend/src/hooks/use-analytics.ts`:

```ts
export function useDailyHeatmap(days = 365, taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'heatmap', days, taskId],
    queryFn: async () => {
      const params = taskId ? `days=${days}&task_id=${taskId}` : `days=${days}`;
      return apiClient<HeatmapEntry[]>(`/api/analytics/daily-heatmap?${params}`);
    },
  });
}

export function useHourlyBreakdown(taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'hourly', taskId],
    queryFn: async () => {
      const url = taskId
        ? `/api/analytics/hourly-breakdown?task_id=${taskId}`
        : '/api/analytics/hourly-breakdown';
      return apiClient<HourlyEntry[]>(url);
    },
  });
}

export function useSessionTools(sessionId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'session-tools', sessionId],
    queryFn: async () =>
      apiClient<SessionToolTokens[]>(`/api/analytics/sessions/${sessionId!}/tools`),
    enabled: !!sessionId,
  });
}
```

Add the new types to the import at the top of the file:
```ts
import type {
  // ... existing ...
  HeatmapEntry, HourlyEntry, SessionToolTokens,
} from '@/types/analytics';
```

- [ ] **Step 5: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/types/analytics.ts frontend/src/lib/pricing.ts \
        frontend/src/hooks/use-analytics.ts \
        frontend/src/components/analytics/command-center.tsx
git commit -m "feat(frontend): add pricing.ts, HeatmapEntry/HourlyEntry types, new analytics hooks"
```

---

## Task 4: Activity Heatmap + Hour-of-Day Chart

**Files:**
- Create: `frontend/src/components/analytics/activity-heatmap.tsx`
- Create: `frontend/src/components/analytics/hourly-breakdown.tsx`

- [ ] **Step 1: Create `activity-heatmap.tsx`**

```tsx
'use client';

import { useState } from 'react';
import { useDailyHeatmap } from '@/hooks/use-analytics';

const COLORS = [
  'hsl(var(--muted))',          // 0 tokens
  '#c7d2fe',                    // indigo-200
  '#818cf8',                    // indigo-400
  '#4f46e5',                    // indigo-600
  '#312e81',                    // indigo-900
];

function tokenColor(tokens: number, max: number): string {
  if (tokens === 0 || max === 0) return COLORS[0];
  const idx = Math.ceil((tokens / max) * 4);
  return COLORS[Math.min(idx, 4)];
}

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

interface Props { taskId?: string | null }

export function ActivityHeatmap({ taskId }: Props) {
  const { data = [], isLoading } = useDailyHeatmap(365, taskId);
  const [tooltip, setTooltip] = useState<{ date: string; tokens: number; x: number; y: number } | null>(null);

  // Build a map for O(1) lookup
  const byDate = new Map(data.map(d => [d.date, d.tokens]));
  const max = Math.max(...data.map(d => d.tokens), 1);

  // Build 52-week grid ending today
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  // Start from the Sunday 52 weeks ago
  const start = new Date(today);
  start.setDate(start.getDate() - 364 - start.getDay());

  const weeks: Array<Array<{ date: string; tokens: number }>> = [];
  const cursor = new Date(start);
  for (let w = 0; w < 53; w++) {
    const week: Array<{ date: string; tokens: number }> = [];
    for (let d = 0; d < 7; d++) {
      const dateStr = cursor.toISOString().slice(0, 10);
      week.push({ date: dateStr, tokens: byDate.get(dateStr) ?? 0 });
      cursor.setDate(cursor.getDate() + 1);
    }
    weeks.push(week);
  }

  const CELL = 11;
  const GAP = 2;
  const stride = CELL + GAP;
  const svgW = weeks.length * stride;
  const svgH = 7 * stride;
  const MONTHS = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  const DAYS = ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <h3 className="font-semibold text-sm">Activity Heatmap</h3>
      {isLoading ? (
        <div className="h-24 animate-pulse bg-muted rounded" />
      ) : (
        <div className="overflow-x-auto relative">
          <div className="flex gap-3 items-start min-w-max">
            {/* Day labels */}
            <svg width={28} height={svgH + 18} className="shrink-0 mt-5">
              {[1, 3, 5].map(d => (
                <text key={d} x={24} y={d * stride + CELL - 1} fontSize={9}
                  fill="hsl(var(--muted-foreground))" textAnchor="end">{DAYS[d]}</text>
              ))}
            </svg>
            {/* Main grid */}
            <div className="relative">
              {/* Month labels */}
              <svg width={svgW} height={14} className="block mb-1">
                {weeks.map((week, wi) => {
                  const month = new Date(week[0].date).getMonth();
                  const prevMonth = wi > 0 ? new Date(weeks[wi - 1][0].date).getMonth() : -1;
                  if (month !== prevMonth) {
                    return <text key={wi} x={wi * stride} y={11} fontSize={9}
                      fill="hsl(var(--muted-foreground))">{MONTHS[month]}</text>;
                  }
                  return null;
                })}
              </svg>
              <svg width={svgW} height={svgH}
                onMouseLeave={() => setTooltip(null)}>
                {weeks.map((week, wi) =>
                  week.map((cell, di) => (
                    <rect
                      key={`${wi}-${di}`}
                      x={wi * stride} y={di * stride}
                      width={CELL} height={CELL}
                      rx={2}
                      fill={tokenColor(cell.tokens, max)}
                      onMouseEnter={(e) => {
                        const rect = (e.target as SVGRectElement).getBoundingClientRect();
                        setTooltip({ date: cell.date, tokens: cell.tokens, x: rect.left, y: rect.top });
                      }}
                    />
                  ))
                )}
              </svg>
              {tooltip && (
                <div className="fixed z-50 pointer-events-none bg-card border border-border rounded px-2 py-1 text-xs shadow"
                  style={{ left: tooltip.x + 14, top: tooltip.y - 30 }}>
                  <span className="font-medium">{tooltip.date}</span>
                  {' — '}
                  {tooltip.tokens > 0 ? `${fmt(tooltip.tokens)} tokens` : 'no activity'}
                </div>
              )}
            </div>
            {/* Legend */}
            <div className="flex flex-col gap-1 mt-5 ml-2 shrink-0">
              <span className="text-[9px] text-muted-foreground">Less</span>
              {COLORS.map((c, i) => (
                <div key={i} className="w-2.5 h-2.5 rounded-[2px]" style={{ background: c }} />
              ))}
              <span className="text-[9px] text-muted-foreground">More</span>
            </div>
          </div>
        </div>
      )}
      <p className="text-[10px] text-muted-foreground">
        Total tokens per day (UTC) · last 365 days · {data.length} active days
      </p>
    </div>
  );
}
```

- [ ] **Step 2: Create `hourly-breakdown.tsx`**

```tsx
'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { useHourlyBreakdown } from '@/hooks/use-analytics';

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

function hourLabel(h: number) {
  if (h === 0) return '12a';
  if (h < 12) return `${h}a`;
  if (h === 12) return '12p';
  return `${h - 12}p`;
}

interface Props { taskId?: string | null }

export function HourlyBreakdown({ taskId }: Props) {
  const { data = [], isLoading } = useHourlyBreakdown(taskId);

  // Fill all 24 hours, default 0
  const filled = Array.from({ length: 24 }, (_, h) => ({
    hour: h,
    label: hourLabel(h),
    tokens: data.find(d => d.hour === h)?.tokens ?? 0,
  }));

  const max = Math.max(...filled.map(d => d.tokens), 1);

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <h3 className="font-semibold text-sm">Activity by Hour</h3>
      {isLoading ? (
        <div className="h-32 animate-pulse bg-muted rounded" />
      ) : (
        <ResponsiveContainer width="100%" height={120}>
          <BarChart data={filled} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
            <XAxis dataKey="label" tick={{ fontSize: 8, fill: 'hsl(var(--muted-foreground))' }}
              interval={2} />
            <YAxis tickFormatter={fmt} tick={{ fontSize: 8, fill: 'hsl(var(--muted-foreground))' }} />
            <Tooltip
              formatter={(v) => [fmt(Number(v)), 'Tokens']}
              labelFormatter={(l) => `Hour: ${l} UTC`}
              contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: 6, fontSize: 11 }}
            />
            <Bar dataKey="tokens" radius={[2, 2, 0, 0]}>
              {filled.map((entry) => (
                <Cell key={entry.hour}
                  fill={`hsl(239 84% ${30 + Math.round((entry.tokens / max) * 35)}%)`} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      )}
      <p className="text-[10px] text-muted-foreground">UTC hours · all time</p>
    </div>
  );
}
```

- [ ] **Step 3: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/analytics/activity-heatmap.tsx \
        frontend/src/components/analytics/hourly-breakdown.tsx
git commit -m "feat(frontend): add ActivityHeatmap and HourlyBreakdown chart components"
```

---

## Task 5: Project Bubble Chart

**Files:**
- Create: `frontend/src/components/analytics/project-bubble-chart.tsx`

- [ ] **Step 1: Create `project-bubble-chart.tsx`**

> **Note:** `TaskTokens` has no `stage` field and the endpoint doesn't return it. Color bubbles by index/rank using a fixed palette — avoids any backend change. Do NOT use a `stageColor()` function.

```tsx
'use client';

import { useState } from 'react';
import { useTokensByTask } from '@/hooks/use-analytics';
import { estimateCost } from '@/lib/pricing';

// Index-based palette — no stage data available from the endpoint
const PALETTE = ['#6366f1','#8b5cf6','#3b82f6','#06b6d4','#10b981','#f59e0b','#ef4444','#64748b'];

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

interface Props {
  onTaskSelect?: (taskId: string) => void;
  selectedTaskId?: string | null;
}

export function ProjectBubbleChart({ onTaskSelect, selectedTaskId }: Props) {
  const { data: tasks = [], isLoading } = useTokensByTask();
  const [hoverId, setHoverId] = useState<string | null>(null);

  const active = tasks.filter(t => t.total_tokens > 0);
  if (isLoading) return <div className="rounded-xl border border-border bg-card p-5 h-48 animate-pulse bg-muted/20" />;
  if (active.length === 0) return null;

  // Sort by total_tokens desc, compute radii
  const sorted = [...active].sort((a, b) => b.total_tokens - a.total_tokens);
  const maxTokens = sorted[0].total_tokens;
  const MIN_R = 20, MAX_R = 80;

  const bubbles = sorted.map((t, i) => ({
    ...t,
    r: MIN_R + Math.sqrt(t.total_tokens / maxTokens) * (MAX_R - MIN_R),
    cost: estimateCost(t.input_tokens, t.output_tokens, t.cache_creation_tokens, t.cache_read_tokens),
    color: PALETTE[i % PALETTE.length],
  }));

  // Simple row-packing layout
  const SVG_W = 800;
  const PAD = 10;
  let x = PAD, y = PAD, rowMaxH = 0;
  const positioned = bubbles.map(b => {
    const diameter = b.r * 2 + 8;
    if (x + diameter > SVG_W - PAD && x > PAD) {
      x = PAD;
      y += rowMaxH + 8;
      rowMaxH = 0;
    }
    const cx = x + b.r;
    const cy = y + b.r;
    x += diameter + 6;
    rowMaxH = Math.max(rowMaxH, b.r * 2);
    return { ...b, cx, cy };
  });
  const svgH = y + rowMaxH + PAD + 20;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Token Allocation by Task</h3>
      </div>
      <div className="overflow-x-auto">
        <svg width="100%" viewBox={`0 0 ${SVG_W} ${svgH}`} style={{ minWidth: 400 }}>
          {positioned.map(b => {
            const isHovered = hoverId === b.task_id;
            const isSelected = selectedTaskId === b.task_id;
            return (
              <g key={b.task_id}
                className="cursor-pointer"
                onClick={() => onTaskSelect?.(b.task_id)}
                onMouseEnter={() => setHoverId(b.task_id)}
                onMouseLeave={() => setHoverId(null)}>
                <circle
                  cx={b.cx} cy={b.cy} r={b.r}
                  fill={b.color}
                  fillOpacity={isHovered ? 0.9 : 0.7}
                  stroke={isSelected ? '#fff' : 'transparent'}
                  strokeWidth={isSelected ? 2.5 : 0}
                  style={{ transition: 'fill-opacity 0.15s' }}
                />
                {b.r > 30 && (
                  <text x={b.cx} y={b.cy - 6} textAnchor="middle" fontSize={Math.min(11, b.r / 4)}
                    fill="white" fontWeight={500}
                    style={{ pointerEvents: 'none' }}>
                    {b.task_title.length > 14 ? b.task_title.slice(0, 13) + '…' : b.task_title}
                  </text>
                )}
                {b.r > 30 && (
                  <text x={b.cx} y={b.cy + 9} textAnchor="middle" fontSize={Math.min(10, b.r / 5)}
                    fill="rgba(255,255,255,0.8)"
                    style={{ pointerEvents: 'none' }}>
                    {fmt(b.total_tokens)}
                  </text>
                )}
                {isHovered && (
                  <title>{`${b.task_title}\n${fmt(b.total_tokens)} tokens · $${b.cost.toFixed(3)}`}</title>
                )}
              </g>
            );
          })}
        </svg>
      </div>
      <p className="text-[10px] text-muted-foreground">
        Bubble size = total tokens · click to filter · {active.length} tasks
      </p>
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/analytics/project-bubble-chart.tsx
git commit -m "feat(frontend): add ProjectBubbleChart component"
```

---

## Task 6: Live Layer — Command Center Upgrades

**Files:**
- Modify: `frontend/src/components/analytics/command-center.tsx`

- [ ] **Step 1: Add live session detection and cost polling**

The live ticker will:
1. On mount, fetch `/api/sessions` to check `active_count > 0`
2. Subscribe to `session_status` WS events to track running sessions
3. When running: refetch `analytics/overview` every 10s via `queryClient.refetchQueries`
4. Show a pulsing `● live` badge next to the cost stat

> **Rate limit note:** The spec suggests 3s polling, but we use **10s** to stay within `/usage` rate limits (per plan-level constraint). Do not change this to 3s.

Add to `command-center.tsx`:

```tsx
import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/contexts/websocket-context';

// Inside CommandCenter():
const queryClient = useQueryClient();
const { subscribe } = useWebSocket();
const [hasRunning, setHasRunning] = useState(false);
const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

// Detect running sessions on mount
useEffect(() => {
  fetch('/api/sessions')
    .then(r => r.json())
    .then((data: { active_count?: number }) => {
      if ((data.active_count ?? 0) > 0) setHasRunning(true);
    })
    .catch(() => {});
}, []);

// Track running sessions via WS
useEffect(() => {
  return subscribe('session_status', (raw: unknown) => {
    const msg = raw as { status?: string };
    if (msg.status === 'running') setHasRunning(true);
    if (msg.status === 'completed' || msg.status === 'failed' || msg.status === 'stopped') {
      // Immediately fetch final cost before potentially clearing live indicator
      queryClient.invalidateQueries({ queryKey: ['analytics', 'overview'] });
      // Check if any others still running before clearing live state
      fetch('/api/sessions')
        .then(r => r.json())
        .then((data: { active_count?: number }) => {
          setHasRunning((data.active_count ?? 0) > 0);
        })
        .catch(() => setHasRunning(false));
    }
  });
}, [subscribe]);

// Poll overview faster when running (10s), respecting rate limits
useEffect(() => {
  if (intervalRef.current) clearInterval(intervalRef.current);
  if (hasRunning) {
    intervalRef.current = setInterval(() => {
      queryClient.invalidateQueries({ queryKey: ['analytics', 'overview'] });
    }, 10_000);
  }
  return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
}, [hasRunning, queryClient]);
```

- [ ] **Step 2: Add live banner and pulsing indicator to the JSX**

In the existing "Total Cost" stat block in `CommandCenter`, add a live indicator:

```tsx
{/* In the Total Cost div */}
<div className="space-y-0.5">
  <div className="flex items-center gap-2">
    <p className="text-xs text-muted-foreground uppercase tracking-wider">Total Cost</p>
    {hasRunning && (
      <span className="flex items-center gap-1 text-[10px] text-amber-500">
        <span className="h-1.5 w-1.5 rounded-full bg-amber-500 animate-pulse" />
        live
      </span>
    )}
  </div>
  {/* ... existing cost display ... */}
</div>
```

Add an active session banner at the top of the CommandCenter card (before the rate limit gauges):

```tsx
{hasRunning && (
  <div className="flex items-center gap-2 text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg px-3 py-2">
    <span className="h-2 w-2 rounded-full bg-amber-500 animate-pulse shrink-0" />
    <span>Session running — cost updating every 10s</span>
  </div>
)}
```

- [ ] **Step 3: Add burn rate sparkline**

Add a `BurnRateSparkline` component inline in `command-center.tsx`:

```tsx
function BurnRateSparkline({ value }: { value: number }) {
  const bufRef = useRef<number[]>(Array(10).fill(0));
  const [points, setPoints] = useState<number[]>(Array(10).fill(0));

  useEffect(() => {
    if (value === 0) return;
    bufRef.current = [...bufRef.current.slice(1), value];
    setPoints([...bufRef.current]);
  }, [value]);

  const max = Math.max(...points, 0.001);
  const W = 60, H = 20;
  const pts = points.map((v, i) =>
    `${(i / 9) * W},${H - (v / max) * H}`
  ).join(' ');

  return (
    <svg width={W} height={H} className="inline-block opacity-70">
      <polyline points={pts} fill="none" stroke="hsl(var(--muted-foreground))" strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  );
}
```

Replace the burn rate text section to include the sparkline:
```tsx
{burnLabel && (
  <div className="flex items-center gap-2 text-xs text-muted-foreground">
    <span>Burn rate: <span className="font-medium text-foreground">{burnLabel}</span></span>
    <BurnRateSparkline value={burn?.tokens_per_minute ?? 0} />
  </div>
)}
```

- [ ] **Step 4: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/analytics/command-center.tsx
git commit -m "feat(frontend): live session banner, polling cost ticker, burn rate sparkline"
```

---

## Task 7: Session Tool Drill-down

**Files:**
- Create: `frontend/src/components/analytics/session-tool-breakdown.tsx`
- Modify: `frontend/src/components/analytics/session-timeline-chart.tsx`

- [ ] **Step 1: Create `session-tool-breakdown.tsx`**

```tsx
'use client';

import { useEffect } from 'react';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { useSessionTools } from '@/hooks/use-analytics';
import { X } from 'lucide-react';

function fmt(n: number) {
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

interface Props {
  sessionId: string;
  sessionTotalTokens: number;
  onClose: () => void;
}

export function SessionToolBreakdown({ sessionId, sessionTotalTokens, onClose }: Props) {
  const { data = [], isLoading } = useSessionTools(sessionId);

  // Data integrity check
  useEffect(() => {
    if (data.length === 0) return;
    const sum = data.reduce((acc, t) => acc + t.input_tokens + t.output_tokens, 0);
    if (Math.abs(sum - sessionTotalTokens) > 10) {
      console.warn(`[SessionToolBreakdown] token mismatch: breakdown=${sum}, session=${sessionTotalTokens}`);
    }
  }, [data, sessionTotalTokens]);

  const chartData = data.map(t => ({
    name: t.tool_name,
    total: t.input_tokens + t.output_tokens,
    calls: t.call_count,
  }));

  const COLORS = ['#6366f1','#8b5cf6','#3b82f6','#06b6d4','#10b981','#f59e0b','#ef4444','#64748b'];

  return (
    <div className="mt-2 rounded-lg border border-border bg-muted/20 p-4 space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          Tool breakdown · session {sessionId.slice(0, 8)}
        </p>
        <button onClick={onClose} className="p-1 rounded hover:bg-muted">
          <X className="h-3.5 w-3.5 text-muted-foreground" />
        </button>
      </div>

      {isLoading ? (
        <div className="h-24 animate-pulse bg-muted rounded" />
      ) : data.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">No tool call data for this session</p>
      ) : (
        <>
          <ResponsiveContainer width="100%" height={Math.max(80, data.length * 28)}>
            <BarChart data={chartData} layout="vertical" margin={{ top: 0, right: 60, left: 10, bottom: 0 }}>
              <XAxis type="number" tickFormatter={fmt} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <YAxis type="category" dataKey="name" tick={{ fontSize: 11, fill: 'hsl(var(--foreground))' }} width={80} />
              <Tooltip
                formatter={(v, _name, props) => [
                  `${fmt(Number(v))} tokens · ${props.payload.calls} calls`,
                  'Total',
                ]}
                contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: 6, fontSize: 11 }}
              />
              <Bar dataKey="total" radius={[0, 3, 3, 0]}>
                {chartData.map((_, i) => <Cell key={i} fill={COLORS[i % COLORS.length]} />)}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
          <div className="flex gap-4 text-xs text-muted-foreground">
            {data.map((t, i) => (
              <span key={t.tool_name} className="flex items-center gap-1">
                <span className="w-2 h-2 rounded-sm" style={{ background: COLORS[i % COLORS.length] }} />
                {t.tool_name}: {t.call_count}×
              </span>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add click handler to `SessionTimelineChart`**

In `frontend/src/components/analytics/session-timeline-chart.tsx`:

1. Add import: `import { SessionToolBreakdown } from './session-tool-breakdown';`
2. Add state: `const [expandedSessionId, setExpandedSessionId] = useState<string | null>(null);`
3. Add click handler on each `Bar` in the chart:

```tsx
<Bar
  dataKey="total_tokens"
  onClick={(data) => {
    setExpandedSessionId(prev =>
      prev === data.session_id ? null : data.session_id
    );
  }}
  style={{ cursor: 'pointer' }}
>
  {barData.map((entry) => (
    <Cell key={entry.session_id} fill={entry.color}
      opacity={expandedSessionId && expandedSessionId !== entry.session_id ? 0.4 : 1} />
  ))}
</Bar>
```

4. After the chart `</ResponsiveContainer>`, render the breakdown panel:

```tsx
{expandedSessionId && (
  <div className="transition-all duration-200">
    <SessionToolBreakdown
      sessionId={expandedSessionId}
      sessionTotalTokens={
        sessions.find(s => s.id === expandedSessionId)?.total_tokens ?? 0
      }
      onClose={() => setExpandedSessionId(null)}
    />
  </div>
)}
```

- [ ] **Step 3: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/analytics/session-tool-breakdown.tsx \
        frontend/src/components/analytics/session-timeline-chart.tsx
git commit -m "feat(frontend): session tool drill-down in SessionTimelineChart"
```

---

## Task 8: Wire Everything into AnalyticsPageInner

**Files:**
- Modify: `frontend/src/components/analytics/analytics-page-inner.tsx`

- [ ] **Step 1: Add WS auto-refresh subscription**

In `AnalyticsPageInner`, add:

```tsx
import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/contexts/websocket-context';

// Inside the component:
const queryClient = useQueryClient();
const { subscribe } = useWebSocket();

useEffect(() => {
  return subscribe('session_status', (raw: unknown) => {
    const msg = raw as { status?: string };
    if (msg.status === 'completed' || msg.status === 'failed') {
      // Invalidate all analytics query keys on session completion.
      // Use exact: false for keys that have taskId/period variants in the cache.
      queryClient.invalidateQueries({ queryKey: ['analytics', 'overview'] });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'burn-rate'] });
      // prefix-match to catch all taskId variants (e.g. ['analytics', 'by-task', 'task-123'])
      queryClient.invalidateQueries({ queryKey: ['analytics', 'by-task'],      exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'cost-by-task'], exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'by-stage'],     exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'by-tool'],      exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'by-language'],  exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'heatmap'],      exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'hourly'],       exact: false });
      // time-series prefix match
      queryClient.invalidateQueries({ queryKey: ['analytics', 'daily'],   exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'weekly'],  exact: false });
      queryClient.invalidateQueries({ queryKey: ['analytics', 'monthly'], exact: false });
    }
  });
}, [subscribe, queryClient]);
```

- [ ] **Step 2: Add new chart imports**

```tsx
import { ActivityHeatmap } from '@/components/analytics/activity-heatmap';
import { HourlyBreakdown } from '@/components/analytics/hourly-breakdown';
import { ProjectBubbleChart } from '@/components/analytics/project-bubble-chart';
```

- [ ] **Step 3: Add heatmap + hourly to Usage Trends section**

In the "Usage Trends" section, before `<TokenTimeChart>`:

```tsx
{/* Heatmap + Hourly */}
<div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
  <div className="lg:col-span-2">
    <ActivityHeatmap taskId={selectedTaskId} />
  </div>
  <HourlyBreakdown taskId={selectedTaskId} />
</div>
```

- [ ] **Step 4: Add bubble chart to ROI section**

In the "ROI & Cost" section, after `<TokensByTaskChart>`:

```tsx
<ProjectBubbleChart
  selectedTaskId={selectedTaskId}
  onTaskSelect={setSelectedTaskId}
/>
```

- [ ] **Step 5: TypeScript check + build**

```bash
cd frontend && npx tsc --noEmit 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/analytics/analytics-page-inner.tsx
git commit -m "feat(frontend): wire analytics v2 — heatmap, hourly, bubble, auto-refresh"
```

---

## Task 9: Verification

- [ ] **Step 1: Run all frontend tests**

```bash
cd frontend && npx vitest run 2>&1 | grep -E "Tests|Test Files"
```
Expected: no new failures introduced by these changes.

- [ ] **Step 2: Run all backend tests**

```bash
cd backend && cargo test 2>&1 | grep "^test result"
```
Expected: all pass.

- [ ] **Step 3: TypeScript full check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: no errors.

- [ ] **Step 4: Visual check** (if app is running)

Open analytics page. Verify:
- Heatmap renders (or shows empty state gracefully)
- Hourly chart renders (24 bars, even if zero)
- Bubble chart visible if tasks have token data
- Command Center shows "live" badge only when a session is running
- Session Timeline: clicking a bar opens the tool breakdown panel
- Clicking same bar again closes it

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: analytics v2 complete — heatmap, hourly, bubble, live layer, drill-down"
```
