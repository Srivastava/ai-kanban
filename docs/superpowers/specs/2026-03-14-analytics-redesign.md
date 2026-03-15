# Analytics Tab Full Redesign

**Date:** 2026-03-14
**Status:** Approved for implementation
**Scope:** Full redesign of the analytics page — OTel bug fix, new backend endpoints, command-center hero, task-filtered dashboard sections.

---

## Goals

1. Fix silent OTel `asInt`-as-string parsing bug (values fall back to 0.0 today)
2. Redesign the analytics page from the ground up into a scrollable command dashboard
3. Add ROI metrics (cost per commit / PR / LOC), productivity data (commits, PRs, active time from OTel), and a plan-tier-aware rate limit display
4. Surface context window usage so users can see when auto-compaction is approaching
5. Add a sticky task selector that cascades filters to all lower sections; Command Center always shows global/account-wide state
6. Ensure every new backend function has tests; OTel fix specifically has a unit test for string-encoded integers

---

## Architecture

### Backend (Rust / Axum / SQLite)

No new tables required. All new data is derived from existing `otel_metrics`, `token_events`, `sessions`, and `tasks` tables.

**Modified files:**
- `backend/src/api/otlp_parser.rs` — fix `asInt` string parsing (`timeUnixNano` string handling already correct in both parsers — no change needed there)
- `backend/src/db/analytics.rs` — add `plan_tier()`, `roi_metrics()`, `context_window_usage()` queries
- `backend/src/models/analytics.rs` — add `PlanTier`, `RoiMetrics`, `ContextWindowUsage` structs
- `backend/src/api/analytics.rs` — add three new route handlers
- `backend/src/api/routes.rs` — register new routes

**New endpoints:**

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/analytics/plan-tier` | Returns configured plan tier and limits — infallible (env-only) |
| GET | `/api/analytics/roi` | Returns cost per commit/PR/LOC + raw counts — returns HTTP 500 `{"error": "..."}` on DB error |
| GET | `/api/analytics/context-usage` | Returns context window state for active sessions — returns HTTP 500 `{"error": "..."}` on DB error |

All error responses follow the existing convention: `(StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))`.

**No changes to `GET /api/analytics/usage-windows`** — the existing endpoint and `UsageWindows` shape are unchanged.

The `RateLimitGauge` component uses both hooks with clear roles:
- `usePlanTier()` — authoritative source for limit values (`limit_5hr`, `limit_week`). Overrides whatever is in `UsageWindows.limit_5hr` / `limit_week` (those existing fields are now deprecated for display purposes but kept for backward compatibility with other consumers).
- `useUsageWindows()` — current consumption only (`tokens_5hr`, `tokens_week`, `reset_5hr`, `reset_week`).

Rule: always use `PlanTier.limit_5hr` as the denominator in the gauge. Ignore `UsageWindows.limit_5hr`.

### Frontend (Next.js / React Query)

**Modified files:**
- `frontend/src/app/analytics/page.tsx` — full rewrite of layout
- `frontend/src/hooks/use-analytics.ts` — add three hooks:
  - `usePlanTier()` — no polling (static/env-driven)
  - `useRoiMetrics(taskId?: string)` — `refetchInterval: 60_000`; always enabled (no `!!taskId` guard — unlike `useDevActivity`, ROI metrics are meaningful globally); appends `?task_id=<taskId>` to the URL only when `taskId` is provided
  - `useContextUsage()` — `refetchInterval: 15_000`
- `frontend/src/types/analytics.ts` — add `PlanTier`, `RoiMetrics`, `ContextWindowUsage` types

**New components** (`frontend/src/components/analytics/`):

| Component | Purpose |
|-----------|---------|
| `command-center.tsx` | Hero section container — always global, never filtered |
| `rate-limit-gauge.tsx` | Reusable progress bar + countdown timer (used for 5hr and weekly) |
| `context-window-gauge.tsx` | Per-session context usage bar |
| `roi-cards.tsx` | Cost per commit / PR / LOC cards |
| `productivity-section.tsx` | Commits + PRs + active time + LOC charts (OTel sourced) |
| `task-filter-bar.tsx` | Sticky searchable task selector; "All Tasks" default |

**Existing components restyled** (not replaced — only `className`, color tokens, and layout wrappers may change; public props and data hooks must remain unchanged):
- `overview-cards.tsx`, `token-time-chart.tsx`, `tool-breakdown-chart.tsx`, `language-chart.tsx`, `stage-breakdown-chart.tsx`, `token-efficiency-chart.tsx`, `session-intelligence-card.tsx`, `cost-breakdown-table.tsx`, `session-timeline-chart.tsx`, `dev-activity-charts.tsx`

---

## Page Layout

```
┌─────────────────────────────────────────────────────────────┐
│  COMMAND CENTER  (always global — no task filter)            │
│                                                              │
│  [Rate Limit: 5hr]  ████████░░░░░  14,200 / 19,000          │
│  Resets in 2h 14m                                            │
│                                                              │
│  [Rate Limit: Weekly]  ███░░░░░░░░  220K / 1M                │
│  Resets Monday 00:00 UTC                                     │
│                                                              │
│  Burn rate: 1,200 tok/hr  •  Limit hit in ~4h 7m             │
│                                                              │
│  [Active sessions with context gauges]                       │
│  Session abc123:  Context 45K / 190K  ████░░░░ (23%)        │
│                                                              │
│  $12.34 total cost  •  4.2M tokens  •  38 sessions          │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  TASK SELECTOR  (sticky below Command Center)                │
│  🔍 [All Tasks ▾]  ← searchable dropdown, most recent first │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  ROI & COST  (responds to task filter)                       │
│  [Cost/Commit]  [Cost/PR]  [Cost/LOC]  ← stat cards        │
│  Cost over time → area chart                                 │
│  Cost by task → horizontal bar chart                         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  USAGE TRENDS  (global; selected task highlighted)           │
│  Daily / Weekly / Monthly  ← tab toggle                      │
│  Area chart — global usage, task overlay as colored band     │
│  Tool breakdown  •  Language breakdown  •  Stage breakdown   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  PRODUCTIVITY  (responds to task filter)                     │
│  Commits + PRs over time (OTel sourced)                      │
│  Active time per task                                        │
│  Lines of code written per session                           │
│  Token efficiency table                                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  SESSION DEEP DIVE  (responds to task filter)                │
│  Session list  →  click to expand timeline chart             │
│  Each row shows: duration, tokens, context usage %           │
│  Dev activity table                                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Backend: OTel Parser Fix

**File:** `backend/src/api/otlp_parser.rs`

**Bug:** `asInt` in OTLP JSON can be a quoted string per the protobuf-JSON encoding spec (int64 → string). Current code: `dp.get("asInt").and_then(|v| v.as_f64())` returns `None` for strings, silently falling back to `0.0`.

**Fix:**
```rust
let value = dp.get("asInt")
    .and_then(|v| v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok())))
    .or_else(|| dp.get("asDouble").and_then(|v| v.as_f64()))
    .unwrap_or(0.0);
```

Note: `timeUnixNano` string-fallback is already correctly implemented in both the metrics and logs parsers — no change needed there.

**New unit test** (`otlp_parser.rs` inline tests):

```rust
// Helper — variant of existing `sample_body` but with asInt as a JSON string
fn sample_body_string_int(metric_name: &str, value_str: &str, session_id: &str) -> Value {
    serde_json::json!({
        "resourceMetrics": [{
            "resource": {"attributes": [
                {"key": "session.id", "value": {"stringValue": session_id}}
            ]},
            "scopeMetrics": [{"metrics": [{
                "name": metric_name,
                "unit": "1",
                "sum": {"dataPoints": [{
                    "attributes": [],
                    "asInt": value_str,   // <-- string-encoded int64
                    "timeUnixNano": "1709000000000000000"
                }]}
            }]}]
        }]
    })
}

#[test]
fn test_parse_asint_as_string() {
    let body = sample_body_string_int("claude_code.commit.count", "42", "sess-abc");
    let results = parse_otlp_metrics(&body);
    assert_eq!(results.len(), 1);
    assert!((results[0].value - 42.0).abs() < 0.01, "asInt string should parse to 42.0");
}

#[test]
fn test_parse_asint_as_number_still_works() {
    let body = sample_body("claude_code.commit.count", 7, "sess-abc");
    let results = parse_otlp_metrics(&body);
    assert_eq!(results.len(), 1);
    assert!((results[0].value - 7.0).abs() < 0.01);
}
```

---

## Backend: Plan Tier

**Implementation:** Pure function (not a DB method) — reads env vars and maps to limits. No database access needed. Lives in a new `backend/src/api/plan_tier.rs` module (or inline in `analytics.rs` handler).

**Env var precedence (explicit):**
1. `CLAUDE_5HR_TOKEN_LIMIT` (existing) — if set and > 0, use it as `limit_5hr`. Similarly `CLAUDE_WEEKLY_TOKEN_LIMIT` for `limit_week`. These user-configured values always win.
2. `CLAUDE_PLAN_TIER` — if `CLAUDE_5HR_TOKEN_LIMIT` is 0 or unset, map the tier name to default limits (`pro` → 19000 / 1000000).
3. Hard-coded Pro defaults — if neither env var is set.

This means existing users who have already configured `CLAUDE_5HR_TOKEN_LIMIT` see no behavior change.

**Default:** `pro` (19,000 tokens / 5hr window, ~1M weekly).
**Override via tier name:** `CLAUDE_PLAN_TIER` env var (`pro` | `max5` | `max20`) — only used when `CLAUDE_5HR_TOKEN_LIMIT` is unset.
**Auto-detect (future):** P90 bucket analysis reserved for later; Pro is the correct default for now.

**Test:** Unit test (not integration) — no DB fixture needed:
```rust
#[test]
fn test_plan_tier_defaults_to_pro() {
    std::env::remove_var("CLAUDE_PLAN_TIER");
    let tier = plan_tier_from_env();
    assert_eq!(tier.tier, "pro");
    assert_eq!(tier.limit_5hr, 19000);
}
```

**Response shape:**
```json
{
  "tier": "pro",
  "limit_5hr": 19000,
  "limit_week": 1000000
}
```

Note: no `detected` field — auto-detection is out of scope. Add it only when P90 detection is implemented.

---

## Backend: ROI Metrics

**Query logic:** Optional `task_id` filter throughout.

- **Cost** (`total_cost_usd`, `avg_session_duration_secs`): from `token_events` joined to `sessions` — same source as existing cost_by_task query.
- **Commits / PRs** (`total_commits`, `total_prs`): `SUM(value)` from `otel_metrics` where `metric_name IN ('claude_code.commit.count', 'claude_code.pull_request.count')` and `task_id IS NOT NULL` (correlated only).
- **LOC** (`total_loc`): from `session_metrics` joined via `sessions` — use `MAX(project_loc) - MIN(CASE WHEN project_loc > 0 THEN project_loc END)` per task, same approach as the existing `dev_activity()` query in `otel_metrics.rs`. Do NOT use `otel_metrics.lines_of_code.count` — it is explicitly unreliable (comment in `otel_metrics.rs` line 72).

**Rust struct:**
```rust
pub struct RoiMetrics {
    pub cost_per_commit: Option<f64>,   // None when total_commits == 0
    pub cost_per_pr: Option<f64>,       // None when total_prs == 0
    pub cost_per_loc: Option<f64>,      // None when total_loc == 0
    pub total_commits: i64,
    pub total_prs: i64,
    pub total_loc: i64,                 // net lines of code written
    pub total_active_time_secs: f64,
    pub avg_session_duration_secs: f64,
    pub total_cost_usd: f64,
}
```

**TypeScript interface:**
```ts
export interface RoiMetrics {
  cost_per_commit: number | null;
  cost_per_pr: number | null;
  cost_per_loc: number | null;
  total_commits: number;
  total_prs: number;
  total_loc: number;
  total_active_time_secs: number;
  avg_session_duration_secs: number;
  total_cost_usd: number;
}
```

**Example response (no commits/PRs yet):**
```json
{
  "cost_per_commit": null,
  "cost_per_pr": null,
  "cost_per_loc": null,
  "total_commits": 0,
  "total_prs": 0,
  "total_loc": 0,
  "total_active_time_secs": 14400.0,
  "avg_session_duration_secs": 1720.0,
  "total_cost_usd": 12.34
}
```

Frontend shows `—` for any `null` field.

---

## Backend: Context Window Usage

For each session with `status = 'running'` (use the literal string `'running'` in SQL — do not use the Rust enum in a sqlx query), compute:
- `tokens_in_window`: effective context size per row = `input_tokens + cache_read_tokens + cache_creation_tokens` (migration 011 added cache columns; bare `input_tokens` grossly underestimates context because most context is in `cache_read_tokens`). Join key: `token_events.session_id = sessions.id` (NOT `claude_session_id`). Compaction reset detection: query `SELECT id, input_tokens + cache_read_tokens + cache_creation_tokens AS ctx FROM token_events WHERE session_id = ? ORDER BY id ASC`; scan rows and find the last row where `ctx < 0.5 * previous_row.ctx` — this is the compaction boundary. Sum `ctx` from that boundary row onwards. If no boundary found (no compaction), sum all `ctx` for the session. If zero rows exist, return `tokens_in_window: 0`. (Threshold 50% is hard-coded; configurable in future via `CLAUDE_COMPACTION_THRESHOLD`.)
- `context_limit`: 190000 (95% of 200K Claude Sonnet context window), configurable via `CLAUDE_CONTEXT_LIMIT` env var
- `pct_used`: `tokens_in_window / context_limit * 100`

Sessions with zero token events (just started) are included with `tokens_in_window: 0` and `pct_used: 0.0` — do not omit them.

Endpoint returns array (one entry per active session):
```json
[
  {
    "session_id": "abc123",
    "task_title": "Add auth middleware",
    "tokens_in_window": 45000,
    "context_limit": 190000,
    "pct_used": 23.7
  }
]
```

---

## Frontend: Task Filter Behavior

- `TaskFilterBar` renders a searchable `<select>` / combobox with "All Tasks" as default
- Selected `taskId` is lifted to the analytics page as local state (`useState`)
- Passed as filter prop to: `RoiSection`, `ProductivitySection`, `SessionDeepDive` — these re-fetch with `?task_id=` when set
- Passed as highlight prop to: `UsageTrendsSection` — fetches global data always, but highlights the selected task's contribution as a colored overlay band on the time chart
- **Not** passed to: `CommandCenter` — always global, no filter, no highlight
- URL param `?task=<id>` synced via `useSearchParams` so a direct link preserves the selection

---

## Frontend: Design Language

- **Color coding for gauges:** green < 60%, amber 60–85%, red > 85%
- **Command Center:** slightly elevated card with a subtle dark gradient background, distinct from the rest of the page
- **Countdown timers:** `HH:MM:SS` format, updating every second via `setInterval` against the reset timestamp from the API
- **Charts:** Recharts (already installed). Area charts for time series, horizontal bars for per-task comparisons, donut/pie for breakdowns
- **OTel-sourced data guard:** if `total_commits === 0 && total_prs === 0`, Productivity section shows a placeholder: *"Productivity data appears once Claude Code reports OTel metrics. Ensure `OTEL_EXPORTER_OTLP_ENDPOINT` points to this server."*
- **Mobile:** all sections stack to single column; task filter bar stays sticky

---

## Testing Strategy

| Layer | What | Where |
|-------|------|-------|
| Unit | `asInt` as string fix | `otlp_parser.rs` inline `#[test]` |
| Unit | `asInt` as numeric still works | `otlp_parser.rs` inline `#[test]` |
| Unit | `plan_tier_from_env()` defaults to pro | `backend/src/api/plan_tier.rs` inline `#[test]` |
| Integration | `roi_metrics()` with and without task filter | `tests/analytics_test.rs` |
| Integration | `context_window_usage()` detects compaction reset | `tests/analytics_test.rs` |
| Integration | `/api/analytics/plan-tier` HTTP route | `tests/analytics_test.rs` |
| Integration | `/api/analytics/roi` HTTP route with task_id filter | `tests/analytics_test.rs` |
| Integration | `/api/analytics/context-usage` HTTP route | `tests/analytics_test.rs` |
| Frontend | `RateLimitGauge` renders correct color at 50%, 75%, 90% | component test |
| Frontend | `TaskFilterBar` URL param sync | component test |

---

## Out of Scope

- Grafana / external dashboard integration
- Multi-user support
- Historical compaction event storage (compaction detection is best-effort from token drop heuristic)
- P90 auto-detect plan tier (Pro default is sufficient; can be added later via `CLAUDE_PLAN_TIER`)
