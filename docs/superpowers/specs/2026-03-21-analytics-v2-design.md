# Analytics v2 — Design Spec

**Date:** 2026-03-21
**Status:** Approved by user
**Approach:** Option B — additive enhancements, all existing charts preserved

---

## Overview

Three independent layers of improvement to the analytics tab:

1. **Live Layer** — real-time updates while Claude sessions run
2. **New Chart Types** — activity heatmap, hour-of-day chart, project bubble chart
3. **Drill-down** — inline tool call breakdown within the Session Timeline

All changes are strictly additive. No existing components are modified in ways that could break current rendering.

---

## 1. Live Layer

### 1.1 Command Center — Live Cost Ticker

**What:** When a session is `running`, the "Total Cost" stat refreshes every 3 seconds, showing cost accrued in real-time.

**How (polling, not WS):**
- The backend has no WS event carrying token counts — `ClaudeEvent::Output` carries display text only; token data is written directly to the DB.
- The live ticker polls `GET /api/analytics/overview` every 3 seconds **only while a running session is detected** (interval paused otherwise).
- Display: `$0.23` (current total) with a pulsing amber dot and label "live" while polling is active.
- On `session_status: completed` WS event, do one final fetch and stop polling. Clear the "live" indicator.

**Data accuracy:** Uses the same server-computed `estimated_cost_usd` as the rest of the page — no client-side approximation.

### 1.2 Active Session Banner

**What:** A pulsing banner inside the Command Center when ≥1 session is running.

**Initialization:** On mount, fetch `GET /api/sessions` to check for sessions with `status: running` or `pending`. This gives correct initial state for pages loaded while a session is already active.

**Updates:** Subscribe to `session_status` WS event to transition to/from active state.

**Display:** `● 1 session running` with amber pulse dot. Disappears when no sessions are running.

### 1.3 Burn Rate Sparkline

**What:** A 10-point mini sparkline next to the burn rate text showing the trend of `tokens_per_minute` over recent polls.

**Data:** The existing `useBurnRate` hook polls `/api/analytics/burn-rate` every 60 seconds and returns `{ tokens_per_minute }`. A `useRef` ring buffer (10 slots) accumulates successive `tokens_per_minute` values as the hook refetches.

**Implementation:** Pure SVG `<polyline>` — no Recharts dependency. Buffer pre-filled with zeros until enough samples arrive. Normalized to max value in the buffer.

### 1.4 Chart Auto-Refresh on Session Complete

**What:** All analytics charts refetch when a session completes, so data updates automatically.

**How:** In `AnalyticsPageInner`, subscribe once to `session_status: completed`. On each completion event, invalidate the following React Query keys:

```
['analytics', 'overview']
['analytics', 'burn-rate']
['analytics', 'daily', *]       (all periods)
['analytics', 'weekly', *]
['analytics', 'monthly', *]
['analytics', 'tokens-by-task']
['analytics', 'cumulative-cost']
['analytics', 'stage-breakdown']
['analytics', 'heatmap', *]
['analytics', 'hourly']
```

One subscription at the page level — charts do not each subscribe independently.

---

## 2. New Chart Types

### 2.1 Activity Heatmap (GitHub-style)

**What:** 52-week × 7-day grid. Each cell = total tokens consumed that calendar day. Color intensity: 5-step indigo scale (empty → light → dark).

**Placement:** New card, full width, at the top of the "Usage Trends" section.

**Backend:**
- New DB method: `daily_heatmap(days: u32) -> Vec<{ date: String, tokens: i64 }>` in `AnalyticsRepository`
- SQL: `SELECT DATE(created_at) as date, SUM(input_tokens + output_tokens) as tokens FROM token_events GROUP BY DATE(created_at) ORDER BY date` with a `days` limit
- New endpoint: `GET /api/analytics/daily-heatmap?days=365` on the existing analytics router
- **Timezone:** UTC only — consistent with all other existing analytics endpoints

**Frontend:**
- New hook `useDailyHeatmap(days: number, taskId?: string | null)` — query key `['analytics', 'heatmap', days, taskId]`
- New component `ActivityHeatmap` — pure SVG grid of `<rect>` elements, 11px × 11px each, 2px gap
- Tooltip on hover: date + formatted token count
- Respects `selectedTaskId` filter (adds `task_id` query param)

### 2.2 Hour-of-Day Breakdown

**What:** 24-bar chart showing total token activity by hour (0–23 UTC).

**Placement:** Paired with the heatmap in a 2-column layout (heatmap 2/3 width, hour chart 1/3 width on large screens; stacked on mobile).

**Backend:**
- New DB method: `hourly_breakdown(task_id: Option<&str>) -> Vec<{ hour: i64, tokens: i64 }>`
- SQL: `SELECT CAST(strftime('%H', created_at) AS INTEGER) as hour, SUM(input_tokens + output_tokens) as tokens FROM token_events GROUP BY hour ORDER BY hour`
- New endpoint: `GET /api/analytics/hourly-breakdown?task_id=...`

**Frontend:**
- New hook `useHourlyBreakdown(taskId?: string | null)` — query key `['analytics', 'hourly', taskId]`
- New component `HourlyBreakdown` — Recharts `BarChart` (already a dependency)

### 2.3 Project Bubble Chart

**What:** Circle-packed bubbles, one per task with token data, sized by total tokens, colored by stage.

**Placement:** New card in the "ROI & Cost" section, full width, below `TokensByTaskChart`.

**Data:** Reuses existing `useTokensByTask()` hook — no new endpoint. Cost computed client-side using same pricing constants as `CommandCenter` (imported from a shared constants file).

**Implementation:** Pure SVG — bubbles sorted by size and laid out in rows. Clicking a bubble calls `onTaskSelect(task_id)` which sets `selectedTaskId` in `AnalyticsPageInner`.

**Accuracy:** Cost label on each bubble uses `(input_tokens / 1_000_000) * 3.0 + (output_tokens / 1_000_000) * 15.0`. Consistent with `CostBreakdownTable`. If the two ever diverge, the inconsistency is a bug to fix — the constants must be defined once and shared.

### 2.4 Shared Pricing Constants

To prevent future cost inconsistencies, extract the pricing constants from `command-center.tsx` into a shared module:

```ts
// frontend/src/lib/pricing.ts
export const PRICING = {
  input:       3.0,   // per 1M tokens
  output:      15.0,
  cacheWrite:  3.75,
  cacheRead:   0.30,
} as const;

export function estimateCost(input: number, output: number, cacheWrite = 0, cacheRead = 0): number {
  return (input / 1_000_000) * PRICING.input
       + (output / 1_000_000) * PRICING.output
       + (cacheWrite / 1_000_000) * PRICING.cacheWrite
       + (cacheRead / 1_000_000) * PRICING.cacheRead;
}
```

`CommandCenter`, `ProjectBubbleChart`, and any future component import from here.

---

## 3. Drill-down: Session Tool Call Breakdown

### 3.1 Inline Tool Breakdown Panel

**What:** In the existing `SessionTimelineChart`, clicking a session bar opens an inline expandable panel showing a per-tool breakdown — horizontal bars for each tool (Read, Edit, Bash, Write, etc.) with token counts and call counts.

**Placement:** Below the clicked bar in the chart. CSS `max-height` transition (0 → auto). Only one panel open at a time.

**Backend:**

New DB method in `AnalyticsRepository`:
```rust
pub async fn tokens_by_tool_for_session(
    &self,
    session_id: &str,
) -> Result<Vec<TokensByTool>>
```
SQL: same as existing `tokens_by_tool` but with `WHERE session_id = ?` instead of optional task filter. **This is a new method** — the existing `tokens_by_tool(task_id)` is not modified.

New endpoint: `GET /api/analytics/sessions/:session_id/tools` on the analytics router.

**Frontend:**
- New hook `useSessionTools(sessionId: string | null)` — query key `['analytics', 'session-tools', sessionId]`, enabled only when `sessionId` is non-null
- New component `SessionToolBreakdown` — horizontal Recharts `BarChart` showing tool breakdown
- Null/no-tool events grouped as "Other"
- Verify client-side: `sum(tool breakdown tokens) === session.total_tokens`. Log warning to console if mismatch (data integrity check, no user-facing error).

---

## New Backend Endpoints Summary

| Method | Path | State type | Purpose |
|--------|------|-----------|---------|
| GET | `/api/analytics/daily-heatmap` | `AnalyticsApiState` | 365-day token heatmap |
| GET | `/api/analytics/hourly-breakdown` | `AnalyticsApiState` | Hour-of-day distribution |
| GET | `/api/analytics/sessions/:id/tools` | `AnalyticsApiState` | Per-session tool breakdown |

All registered in the existing `analytics_routes()` function in `backend/src/api/analytics.rs`.

---

## New Frontend Files

| File | Type | Notes |
|------|------|-------|
| `src/lib/pricing.ts` | Utility | Shared pricing constants + `estimateCost()` |
| `src/components/analytics/activity-heatmap.tsx` | Component | Pure SVG |
| `src/components/analytics/hourly-breakdown.tsx` | Component | Recharts |
| `src/components/analytics/project-bubble-chart.tsx` | Component | Pure SVG |
| `src/components/analytics/session-tool-breakdown.tsx` | Component | Recharts |

Modified files:
| File | Change |
|------|--------|
| `src/components/analytics/command-center.tsx` | Add live banner, polling ticker, sparkline |
| `src/components/analytics/analytics-page-inner.tsx` | Add WS invalidation subscription, new sections |
| `src/components/analytics/session-timeline-chart.tsx` | Add click handler → drill-down panel |
| `src/hooks/use-analytics.ts` | Add `useDailyHeatmap`, `useHourlyBreakdown`, `useSessionTools` |

---

## What Is NOT Changed

- `TokenTimeChart`, `ToolBreakdownChart`, `LanguageChart`, `StageBreakdownChart`, `CostBreakdownTable`, `CumulativeCostChart`, `SessionIntelligenceCard`, `RoiCards` — untouched
- No new chart libraries added
- No DB schema changes or migrations needed
- No changes to existing backend API contracts
