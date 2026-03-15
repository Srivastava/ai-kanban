# Analytics Redesign Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Full analytics page redesign — fix OTel `asInt` bug, add plan-tier-aware rate limit gauges, ROI metrics, context window usage, and a sticky task filter that cascades to all lower sections.

**Architecture:** Backend adds three new endpoints (plan-tier, roi, context-usage) plus an OTel parser fix, all in existing files plus one new module. Frontend fully rewrites `analytics/page.tsx` with a Command Center hero section plus new components; existing components get className-only restyling.

**Tech Stack:** Rust/Axum/SQLite/sqlx, Next.js 14, React Query, Recharts, Tailwind CSS

**Spec:** `docs/superpowers/specs/2026-03-14-analytics-redesign.md`

---

## File Map

### Backend — create
| File | Responsibility |
|------|---------------|
| `backend/src/api/plan_tier.rs` | Pure fn `plan_tier_from_env()` — reads env vars, returns `PlanTier` |

### Backend — modify
| File | Changes |
|------|---------|
| `backend/src/api/otlp_parser.rs` | Fix `asInt` string parsing + two new unit tests |
| `backend/src/models/analytics.rs` | Add `PlanTier`, `RoiMetrics`, `ContextWindowUsage` structs |
| `backend/src/db/analytics.rs` | Add `roi_metrics(task_id)` + `context_window_usage()` methods |
| `backend/src/api/analytics.rs` | Add three handlers; extend `analytics_routes()` |
| `backend/src/api/mod.rs` | Add `pub mod plan_tier;` |
| `backend/tests/analytics_extended_test.rs` | New integration tests for roi + context-usage endpoints |

### Frontend — create
| File | Responsibility |
|------|---------------|
| `frontend/src/components/analytics/task-filter-bar.tsx` | Searchable task selector; syncs `?task` URL param |
| `frontend/src/components/analytics/rate-limit-gauge.tsx` | Reusable progress bar + countdown (5hr or weekly) |
| `frontend/src/components/analytics/context-window-gauge.tsx` | Per-session context usage bar |
| `frontend/src/components/analytics/command-center.tsx` | Hero section — assembles gauges + headline stats |
| `frontend/src/components/analytics/roi-cards.tsx` | Cost/commit, cost/PR, cost/LOC stat cards |
| `frontend/src/components/analytics/productivity-section.tsx` | Commits, PRs, active time, LOC from OTel |

### Frontend — modify
| File | Changes |
|------|---------|
| `frontend/src/types/analytics.ts` | Add `PlanTier`, `RoiMetrics`, `ContextWindowUsage` |
| `frontend/src/hooks/use-analytics.ts` | Add `usePlanTier()`, `useRoiMetrics(taskId?)`, `useContextUsage()` |
| `frontend/src/app/analytics/page.tsx` | Full rewrite — new section layout with task filter |
| Existing analytics components (10 files) | className + color token changes only |

---

## Chunk 1: Backend — OTel `asInt` Fix + Plan Tier

### Task 1: Fix `asInt` string parsing in `otlp_parser.rs`

**Files:**
- Modify: `backend/src/api/otlp_parser.rs`

- [ ] **Step 1: Write the two failing unit tests**

Add inside the existing `#[cfg(test)]` block in `backend/src/api/otlp_parser.rs`, after the existing `sample_body` helper:

```rust
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
                    "asInt": value_str,
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
    assert!((results[0].value - 42.0).abs() < 0.01, "expected 42.0 got {}", results[0].value);
}

#[test]
fn test_parse_asint_as_number_still_works() {
    let body = sample_body("claude_code.commit.count", 7, "sess-abc");
    let results = parse_otlp_metrics(&body);
    assert_eq!(results.len(), 1);
    assert!((results[0].value - 7.0).abs() < 0.01);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test parse_asint -- --nocapture 2>&1 | tail -20
```

Expected: `test_parse_asint_as_string` FAILS (value is 0.0, not 42.0). `test_parse_asint_as_number_still_works` PASSES.

- [ ] **Step 3: Apply the fix**

In `backend/src/api/otlp_parser.rs`, find lines 60–63 (the `value` extraction):

```rust
// BEFORE:
let value = dp.get("asInt").and_then(|v| v.as_f64())
    .or_else(|| dp.get("asDouble").and_then(|v| v.as_f64()))
    .unwrap_or(0.0);
```

Replace with:

```rust
// AFTER:
let value = dp.get("asInt")
    .and_then(|v| v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok())))
    .or_else(|| dp.get("asDouble").and_then(|v| v.as_f64()))
    .unwrap_or(0.0);
```

- [ ] **Step 4: Run tests — both must pass**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test parse_asint -- --nocapture 2>&1 | tail -10
```

Expected: 2 tests PASS.

- [ ] **Step 5: Run full test suite to check no regressions**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/api/otlp_parser.rs && git commit -m "fix: parse OTLP asInt when encoded as quoted string

OTLP JSON spec allows int64 fields to arrive as quoted strings.
Add string-parse fallback so values like \"asInt\": \"42\" are
correctly read as 42.0 instead of silently falling back to 0.0.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Add `PlanTier` model and `plan_tier_from_env()` pure function

**Files:**
- Modify: `backend/src/models/analytics.rs`
- Create: `backend/src/api/plan_tier.rs`
- Modify: `backend/src/api/mod.rs`

- [ ] **Step 1: Add `PlanTier` struct to models**

Append to `backend/src/models/analytics.rs`:

```rust
#[derive(Debug, Serialize)]
pub struct PlanTier {
    pub tier: String,
    pub limit_5hr: i64,
    pub limit_week: i64,
}
```

- [ ] **Step 2: Write the unit test first (in the new file)**

Create `backend/src/api/plan_tier.rs`:

```rust
/// Returns the active Claude plan tier and its rate-limit values.
///
/// Precedence:
///   1. CLAUDE_5HR_TOKEN_LIMIT env var (> 0) — user-configured, always wins
///   2. CLAUDE_PLAN_TIER env var ("pro" | "max5" | "max20")
///   3. Hard-coded Pro defaults
pub fn plan_tier_from_env() -> crate::models::PlanTier {
    // Check for user-configured explicit limits first
    let explicit_5hr: i64 = std::env::var("CLAUDE_5HR_TOKEN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let explicit_week: i64 = std::env::var("CLAUDE_WEEKLY_TOKEN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if explicit_5hr > 0 || explicit_week > 0 {
        return crate::models::PlanTier {
            tier: "custom".to_string(),
            limit_5hr: explicit_5hr,
            limit_week: explicit_week,
        };
    }

    // Map tier name to defaults
    let tier_name = std::env::var("CLAUDE_PLAN_TIER")
        .unwrap_or_else(|_| "pro".to_string())
        .to_lowercase();

    match tier_name.as_str() {
        "max5"  => crate::models::PlanTier { tier: "max5".to_string(),  limit_5hr: 88_000,   limit_week: 5_000_000  },
        "max20" => crate::models::PlanTier { tier: "max20".to_string(), limit_5hr: 220_000,  limit_week: 20_000_000 },
        _       => crate::models::PlanTier { tier: "pro".to_string(),   limit_5hr: 19_000,   limit_week: 1_000_000  },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_to_pro() {
        std::env::remove_var("CLAUDE_PLAN_TIER");
        std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
        std::env::remove_var("CLAUDE_WEEKLY_TOKEN_LIMIT");
        let t = plan_tier_from_env();
        assert_eq!(t.tier, "pro");
        assert_eq!(t.limit_5hr, 19_000);
        assert_eq!(t.limit_week, 1_000_000);
    }

    #[test]
    fn test_explicit_5hr_limit_wins() {
        std::env::set_var("CLAUDE_5HR_TOKEN_LIMIT", "50000");
        std::env::remove_var("CLAUDE_PLAN_TIER");
        let t = plan_tier_from_env();
        assert_eq!(t.tier, "custom");
        assert_eq!(t.limit_5hr, 50_000);
        std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
    }

    #[test]
    fn test_max5_tier() {
        std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
        std::env::remove_var("CLAUDE_WEEKLY_TOKEN_LIMIT");
        std::env::set_var("CLAUDE_PLAN_TIER", "max5");
        let t = plan_tier_from_env();
        assert_eq!(t.tier, "max5");
        assert_eq!(t.limit_5hr, 88_000);
        std::env::remove_var("CLAUDE_PLAN_TIER");
    }
}
```

- [ ] **Step 3: Expose the module**

In `backend/src/api/mod.rs`, add after the existing `pub mod` declarations:

```rust
pub mod plan_tier;
```

- [ ] **Step 4: Run tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test plan_tier -- --nocapture 2>&1 | tail -15
```

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/models/analytics.rs backend/src/api/plan_tier.rs backend/src/api/mod.rs && git commit -m "feat: add PlanTier model and plan_tier_from_env() pure function

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Wire `/api/analytics/plan-tier` HTTP endpoint

**Files:**
- Modify: `backend/src/api/analytics.rs`

- [ ] **Step 1: Add handler and route**

In `backend/src/api/analytics.rs`, add the handler before the closing brace of the file:

```rust
async fn plan_tier_handler() -> impl IntoResponse {
    info!("API: Getting plan tier");
    Json(crate::api::plan_tier::plan_tier_from_env()).into_response()
}
```

In `analytics_routes()`, add the new route (the function already exists):

```rust
.route("/plan-tier", get(plan_tier_handler))
```

- [ ] **Step 2: Build to check it compiles**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep -E "error|warning" | head -20
```

Expected: compiles cleanly (no errors).

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/api/analytics.rs && git commit -m "feat: add GET /api/analytics/plan-tier endpoint

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 2: Backend — ROI Metrics

### Task 4: Add `RoiMetrics` model

**Files:**
- Modify: `backend/src/models/analytics.rs`

- [ ] **Step 1: Append struct**

```rust
#[derive(Debug, Serialize)]
pub struct RoiMetrics {
    pub cost_per_commit:   Option<f64>,
    pub cost_per_pr:       Option<f64>,
    pub cost_per_loc:      Option<f64>,
    pub total_commits:     i64,
    pub total_prs:         i64,
    pub total_loc:         i64,
    pub total_active_time_secs: f64,
    pub avg_session_duration_secs: f64,
    pub total_cost_usd:    f64,
}
```

- [ ] **Step 2: Build**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep "^error" | head -5
```

Expected: no errors.

---

### Task 5: Implement `roi_metrics()` DB query

**Files:**
- Modify: `backend/src/db/analytics.rs`

- [ ] **Step 1: Write the integration test first**

Open `backend/tests/analytics_extended_test.rs`. Add the following test (check if file already has a setup helper — if so, reuse it; otherwise add the one below):

```rust
// Add ONLY these missing imports to the existing `use ai_kanban_backend::db::{...}` line at the top:
//   OtelMetricsRepository
// Add ONLY these missing imports to the existing `use ai_kanban_backend::models::{...}` line:
//   CreateOtelMetric
// Do NOT duplicate items already in the existing use declarations (AnalyticsRepository,
// SessionRepository, TaskRepository, TokenEventRepository, create_pool are already there).

async fn setup_roi_db() -> (sqlx::SqlitePool, TaskRepository, SessionRepository, TokenEventRepository, OtelMetricsRepository, AnalyticsRepository) {
    let pool = create_pool(":memory:").await.unwrap();
    (
        pool.clone(),
        TaskRepository::new(pool.clone()),
        SessionRepository::new(pool.clone()),
        TokenEventRepository::new(pool.clone()),
        OtelMetricsRepository::new(pool.clone()),
        AnalyticsRepository::new(pool.clone()),
    )
}

#[tokio::test]
async fn test_roi_metrics_no_data() {
    let (_, _, _, _, _, analytics) = setup_roi_db().await;
    let roi = analytics.roi_metrics(None).await.unwrap();
    assert_eq!(roi.total_commits, 0);
    assert_eq!(roi.total_prs, 0);
    assert!(roi.cost_per_commit.is_none());
    assert!(roi.cost_per_pr.is_none());
    assert_eq!(roi.total_cost_usd, 0.0);
}

#[tokio::test]
async fn test_roi_metrics_with_data() {
    let (pool, task_repo, session_repo, event_repo, otel_repo, analytics) = setup_roi_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test Task".to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    // Seed token event (100 input, 50 output)
    event_repo.create(CreateTokenEvent {
        session_id: session.id.clone(),
        task_id: task.id.clone(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 100_000,
        output_tokens: 50_000,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        model: None,
        sequence_no: Some(0),
    }).await.unwrap();

    // Seed commit + PR OTel metrics
    for (name, val) in [("claude_code.commit.count", 3.0), ("claude_code.pull_request.count", 1.0)] {
        otel_repo.insert(CreateOtelMetric {
            metric_name: name.to_string(),
            value: val,
            unit: None,
            session_id: Some(session.id.clone()),
            task_id: Some(task.id.clone()),
            claude_session_id: "cs-abc".to_string(),
            attributes: serde_json::json!({}),
            otel_timestamp: 1_709_000_000_000_000_000,
        }).await.unwrap();
    }

    let roi = analytics.roi_metrics(None).await.unwrap();
    assert_eq!(roi.total_commits, 3);
    assert_eq!(roi.total_prs, 1);
    assert!(roi.cost_per_commit.is_some());
    assert!(roi.cost_per_pr.is_some());
    // 100K input @ $3/M = $0.30; 50K output @ $15/M = $0.75; total = $1.05
    assert!((roi.total_cost_usd - 1.05).abs() < 0.01, "expected ~1.05, got {}", roi.total_cost_usd);
    // cost_per_commit = 1.05 / 3 ≈ 0.35
    assert!((roi.cost_per_commit.unwrap() - 0.35).abs() < 0.01);
}

#[tokio::test]
async fn test_roi_metrics_task_filter() {
    let (pool, task_repo, session_repo, event_repo, otel_repo, analytics) = setup_roi_db().await;

    // Create two tasks
    let t1 = task_repo.create(CreateTask { title: "T1".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let t2 = task_repo.create(CreateTask { title: "T2".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let s1 = session_repo.create(CreateSession { task_id: t1.id.clone() }).await.unwrap();
    let s2 = session_repo.create(CreateSession { task_id: t2.id.clone() }).await.unwrap();

    for (tid, sid, commits) in [(&t1.id, &s1.id, 2.0), (&t2.id, &s2.id, 5.0)] {
        otel_repo.insert(CreateOtelMetric {
            metric_name: "claude_code.commit.count".to_string(),
            value: commits,
            unit: None,
            session_id: Some(sid.clone()),
            task_id: Some(tid.clone()),
            claude_session_id: "cs-x".to_string(),
            attributes: serde_json::json!({}),
            otel_timestamp: 1_709_000_000_000_000_000,
        }).await.unwrap();
    }

    let all = analytics.roi_metrics(None).await.unwrap();
    assert_eq!(all.total_commits, 7); // 2 + 5

    let filtered = analytics.roi_metrics(Some(&t1.id)).await.unwrap();
    assert_eq!(filtered.total_commits, 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test roi_metrics -- --nocapture 2>&1 | tail -10
```

Expected: compile error (method doesn't exist yet).

- [ ] **Step 3: Implement `roi_metrics()` in `backend/src/db/analytics.rs`**

Add this method to `AnalyticsRepository` (after the existing methods):

```rust
pub async fn roi_metrics(&self, task_id: Option<&str>) -> Result<crate::models::RoiMetrics> {
    // token_prices() is defined in this same file — call directly, no import needed
    // ── Cost + duration from token_events ───────────────────────────────
    let (input_price, output_price) = token_prices();

    let cost_row = sqlx::query(r#"
        SELECT
            COALESCE(SUM(te.input_tokens), 0)  AS total_input,
            COALESCE(SUM(te.output_tokens), 0) AS total_output,
            COUNT(DISTINCT te.session_id)       AS session_count,
            COALESCE(
                AVG(CASE WHEN s.ended_at IS NOT NULL
                    THEN CAST((julianday(s.ended_at) - julianday(s.started_at)) * 86400.0 AS REAL)
                    END),
                0.0
            ) AS avg_duration_secs
        FROM token_events te
        JOIN sessions s ON s.id = te.session_id
        WHERE (? IS NULL OR te.task_id = ?)
    "#)
    .bind(task_id)
    .bind(task_id)
    .fetch_one(&self.pool)
    .await?;

    let total_input:  f64 = cost_row.get::<i64, _>("total_input")  as f64;
    let total_output: f64 = cost_row.get::<i64, _>("total_output") as f64;
    let avg_duration: f64 = cost_row.get("avg_duration_secs");
    let total_cost = (total_input / 1_000_000.0) * input_price
                   + (total_output / 1_000_000.0) * output_price;

    // ── Commits / PRs / active time from otel_metrics ───────────────────
    let otel_row = sqlx::query(r#"
        SELECT
            COALESCE(SUM(CASE WHEN metric_name = 'claude_code.commit.count'
                         THEN value ELSE 0 END), 0) AS total_commits,
            COALESCE(SUM(CASE WHEN metric_name = 'claude_code.pull_request.count'
                         THEN value ELSE 0 END), 0) AS total_prs,
            COALESCE(SUM(CASE WHEN metric_name = 'claude_code.active_time.total'
                         THEN value ELSE 0 END), 0.0) AS total_active_time
        FROM otel_metrics
        WHERE task_id IS NOT NULL
          AND (? IS NULL OR task_id = ?)
    "#)
    .bind(task_id)
    .bind(task_id)
    .fetch_one(&self.pool)
    .await?;

    let total_commits: i64 = {
        let v: f64 = otel_row.get("total_commits");
        v as i64
    };
    let total_prs: i64 = {
        let v: f64 = otel_row.get("total_prs");
        v as i64
    };
    let total_active: f64 = otel_row.get("total_active_time");

    // ── LOC from session_metrics (same source as dev_activity) ──────────
    let loc_row = sqlx::query(r#"
        SELECT
            COALESCE(MAX(sm.project_loc), 0)
            - COALESCE(MIN(CASE WHEN sm.project_loc > 0 THEN sm.project_loc END), 0)
            AS net_loc
        FROM session_metrics sm
        JOIN sessions s ON s.id = sm.session_id
        WHERE (? IS NULL OR s.task_id = ?)
    "#)
    .bind(task_id)
    .bind(task_id)
    .fetch_one(&self.pool)
    .await?;

    // SQLite returns arithmetic expressions as REAL — decode as f64, cast to i64
    let total_loc: i64 = {
        let v: f64 = loc_row.get("net_loc");
        v as i64
    };

    Ok(crate::models::RoiMetrics {
        cost_per_commit:  if total_commits > 0 { Some(total_cost / total_commits as f64) } else { None },
        cost_per_pr:      if total_prs > 0     { Some(total_cost / total_prs as f64)     } else { None },
        cost_per_loc:     if total_loc > 0     { Some(total_cost / total_loc as f64)     } else { None },
        total_commits,
        total_prs,
        total_loc,
        total_active_time_secs: total_active,
        avg_session_duration_secs: avg_duration,
        total_cost_usd: total_cost,
    })
}
```

- [ ] **Step 4: Run tests — all three must pass**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test roi_metrics -- --nocapture 2>&1 | tail -15
```

Expected: 3 tests PASS.

---

### Task 6: Wire `/api/analytics/roi` HTTP endpoint

**Files:**
- Modify: `backend/src/api/analytics.rs`

- [ ] **Step 1: Add handler**

```rust
#[derive(Deserialize, Debug)]
struct RoiQuery {
    task_id: Option<String>,
}

#[instrument(skip(state))]
async fn roi_metrics_handler(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<RoiQuery>,
) -> impl IntoResponse {
    info!(task_id = ?query.task_id, "API: Getting ROI metrics");
    match state.analytics.roi_metrics(query.task_id.as_deref()).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "API: Failed to get ROI metrics");
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}
```

Add to `analytics_routes()`:

```rust
.route("/roi", get(roi_metrics_handler))
```

- [ ] **Step 2: Build**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep "^error" | head -5
```

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/models/analytics.rs backend/src/db/analytics.rs backend/src/api/analytics.rs backend/tests/analytics_extended_test.rs && git commit -m "feat: add ROI metrics — cost/commit, cost/PR, cost/LOC endpoints

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 3: Backend — Context Window Usage

### Task 7: Add `ContextWindowUsage` model

**Files:**
- Modify: `backend/src/models/analytics.rs`

- [ ] **Step 1: Append struct**

```rust
#[derive(Debug, Serialize)]
pub struct ContextWindowUsage {
    pub session_id:       String,
    pub task_title:       String,
    pub tokens_in_window: i64,
    pub context_limit:    i64,
    pub pct_used:         f64,
}
```

---

### Task 8: Implement `context_window_usage()` DB query

**Files:**
- Modify: `backend/src/db/analytics.rs`

- [ ] **Step 1: Write the integration test first**

Append to `backend/tests/analytics_extended_test.rs`:

```rust
#[tokio::test]
async fn test_context_window_usage_empty_when_no_running_sessions() {
    let (_, _, _, _, _, analytics) = setup_roi_db().await;
    let result = analytics.context_window_usage().await.unwrap();
    assert!(result.is_empty(), "no running sessions → empty result");
}

#[tokio::test]
async fn test_context_window_usage_running_session() {
    let (pool, task_repo, session_repo, event_repo, _, analytics) = setup_roi_db().await;

    let task = task_repo.create(CreateTask {
        title: "Running Task".to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    // Mark session as running
    sqlx::query("UPDATE sessions SET status = 'running' WHERE id = ?")
        .bind(&session.id)
        .execute(&pool)
        .await
        .unwrap();

    // Add token events: 50K input, 30K cache_read
    event_repo.create(CreateTokenEvent {
        session_id: session.id.clone(),
        task_id: task.id.clone(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 50_000,
        output_tokens: 10_000,
        cache_read_tokens: 30_000,
        cache_creation_tokens: 5_000,
        model: None,
        sequence_no: Some(0),
    }).await.unwrap();

    let result = analytics.context_window_usage().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].task_title, "Running Task");
    // tokens_in_window = 50_000 + 30_000 + 5_000 = 85_000
    assert_eq!(result[0].tokens_in_window, 85_000);
    let limit = result[0].context_limit;
    assert!(limit > 0);
    let pct = result[0].pct_used;
    assert!((pct - (85_000.0 / limit as f64 * 100.0)).abs() < 0.1);
}
```

- [ ] **Step 2: Run tests to verify they fail (compile error)**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test context_window -- --nocapture 2>&1 | tail -10
```

Expected: compile error — method not found.

- [ ] **Step 3: Implement `context_window_usage()`**

Add to `AnalyticsRepository` in `backend/src/db/analytics.rs`:

```rust
pub async fn context_window_usage(&self) -> Result<Vec<crate::models::ContextWindowUsage>> {
    let context_limit: i64 = std::env::var("CLAUDE_CONTEXT_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(190_000);

    // Fetch all running sessions with their task titles
    let sessions = sqlx::query(
        r#"SELECT s.id AS session_id, t.title AS task_title
           FROM sessions s
           JOIN tasks t ON t.id = s.task_id
           WHERE s.status = 'running'
           ORDER BY s.started_at DESC"#
    )
    .fetch_all(&self.pool)
    .await?;

    let mut result = Vec::new();

    for row in sessions {
        let session_id: String = row.get("session_id");
        let task_title: String = row.get("task_title");

        // Fetch token events for this session ordered by id ASC
        let events = sqlx::query(
            r#"SELECT input_tokens + cache_read_tokens + cache_creation_tokens AS ctx
               FROM token_events
               WHERE session_id = ?
               ORDER BY id ASC"#
        )
        .bind(&session_id)
        .fetch_all(&self.pool)
        .await?;

        // Find last compaction boundary (ctx < 50% of previous)
        let mut boundary_idx = 0usize;
        let ctxs: Vec<i64> = events.iter()
            .map(|r| r.get::<i64, _>("ctx"))
            .collect();

        for i in 1..ctxs.len() {
            if ctxs[i] < ctxs[i - 1] / 2 {
                boundary_idx = i;
            }
        }

        let tokens_in_window: i64 = ctxs[boundary_idx..].iter().sum();
        let pct_used = if context_limit > 0 {
            tokens_in_window as f64 / context_limit as f64 * 100.0
        } else {
            0.0
        };

        result.push(crate::models::ContextWindowUsage {
            session_id,
            task_title,
            tokens_in_window,
            context_limit,
            pct_used,
        });
    }

    Ok(result)
}
```

- [ ] **Step 4: Run tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test context_window -- --nocapture 2>&1 | tail -15
```

Expected: 2 tests PASS.

---

### Task 9: Wire `/api/analytics/context-usage` HTTP endpoint

**Files:**
- Modify: `backend/src/api/analytics.rs`

- [ ] **Step 1: Add handler**

```rust
#[instrument(skip(state))]
async fn context_usage_handler(
    State(state): State<AnalyticsApiState>,
) -> impl IntoResponse {
    info!("API: Getting context window usage");
    match state.analytics.context_window_usage().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "API: Failed to get context usage");
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}
```

Add to `analytics_routes()`:

```rust
.route("/context-usage", get(context_usage_handler))
```

Also update `AnalyticsApiState` — the `context_window_usage()` method needs to be on `AnalyticsRepository` which is already in `state.analytics`. No struct changes needed.

- [ ] **Step 2: Add HTTP route integration tests**

Add to `backend/tests/api_test.rs` (which already has `setup_test_server()` and the correct imports). Append after the existing tests:

```rust
// ==================== Analytics New Endpoints ====================

#[tokio::test]
async fn test_analytics_plan_tier_endpoint() {
    // Clear env vars so we get a deterministic "pro" default
    std::env::remove_var("CLAUDE_PLAN_TIER");
    std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
    std::env::remove_var("CLAUDE_WEEKLY_TOKEN_LIMIT");

    let server = setup_test_server().await;
    let resp = server.get("/api/analytics/plan-tier").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["tier"].is_string());
    assert!(body["limit_5hr"].as_i64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn test_analytics_roi_endpoint() {
    let server = setup_test_server().await;
    let resp = server.get("/api/analytics/roi").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["total_commits"].is_number());
    assert!(body["total_cost_usd"].is_number());
}

#[tokio::test]
async fn test_analytics_context_usage_endpoint() {
    let server = setup_test_server().await;
    let resp = server.get("/api/analytics/context-usage").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body.is_array(), "context-usage should return an array");
}
```

Note: `setup_test_server()` and `StatusCode` (from `axum_test::http::StatusCode`) are already imported at the top of `api_test.rs`. No new imports needed.

- [ ] **Step 3: Run all backend tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/models/analytics.rs backend/src/db/analytics.rs backend/src/api/analytics.rs backend/tests/analytics_extended_test.rs && git commit -m "feat: add context-window-usage endpoint + integration tests

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 4: Frontend — Types, Hooks, TaskFilterBar

### Task 10: Add new TypeScript types

**Files:**
- Modify: `frontend/src/types/analytics.ts`

- [ ] **Step 1: Append to `frontend/src/types/analytics.ts`**

```ts
export interface PlanTier {
  tier: string;
  limit_5hr: number;
  limit_week: number;
}

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

export interface ContextWindowUsage {
  session_id: string;
  task_title: string;
  tokens_in_window: number;
  context_limit: number;
  pct_used: number;
}
```

---

### Task 11: Add new hooks

**Files:**
- Modify: `frontend/src/hooks/use-analytics.ts`

- [ ] **Step 1: Append three hooks**

```ts
import type { ..., PlanTier, RoiMetrics, ContextWindowUsage } from '@/types/analytics';
// Add the three new types to the existing import line

export function usePlanTier() {
  return useQuery({
    queryKey: ['analytics', 'plan-tier'],
    queryFn: () => apiClient<PlanTier>('/api/analytics/plan-tier'),
    // No refetchInterval — static/env-driven
  });
}

export function useRoiMetrics(taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'roi', taskId],
    queryFn: () => apiClient<RoiMetrics>(
      taskId ? `/api/analytics/roi?task_id=${taskId}` : '/api/analytics/roi'
    ),
    refetchInterval: 60_000,
    // Always enabled — ROI is meaningful globally (no !!taskId guard)
  });
}

export function useContextUsage() {
  return useQuery({
    queryKey: ['analytics', 'context-usage'],
    queryFn: () => apiClient<ContextWindowUsage[]>('/api/analytics/context-usage'),
    refetchInterval: 15_000,
  });
}
```

- [ ] **Step 2: Build to check for TS errors**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

---

### Task 12: Build `TaskFilterBar` component

**Files:**
- Create: `frontend/src/components/analytics/task-filter-bar.tsx`

- [ ] **Step 1: Create the component**

```tsx
'use client';

import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';

interface TaskOption {
  id: string;
  title: string;
}

interface Props {
  selectedTaskId: string | null;
  onSelect: (taskId: string | null) => void;
}

export function TaskFilterBar({ selectedTaskId, onSelect }: Props) {
  const { data: tasks = [] } = useQuery({
    queryKey: ['tasks', 'for-filter'],
    queryFn: () => apiClient<TaskOption[]>('/api/tasks'),
    select: (data: any[]) => data
      .map((t: any) => ({ id: t.id, title: t.title }))
      .slice(0, 50), // cap for dropdown size
  });

  return (
    <div className="sticky top-0 z-10 border-b border-border bg-background/95 backdrop-blur px-4 sm:px-6 py-3 flex items-center gap-3">
      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider shrink-0">
        Filter by task
      </span>
      <select
        value={selectedTaskId ?? ''}
        onChange={(e) => onSelect(e.target.value || null)}
        className="flex-1 max-w-xs rounded-md border border-border bg-card px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
      >
        <option value="">All Tasks</option>
        {tasks.map((t) => (
          <option key={t.id} value={t.id}>{t.title}</option>
        ))}
      </select>
      {selectedTaskId && (
        <button
          onClick={() => onSelect(null)}
          className="text-xs text-muted-foreground hover:text-foreground underline"
        >
          Clear
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: TS check**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add frontend/src/types/analytics.ts frontend/src/hooks/use-analytics.ts frontend/src/components/analytics/task-filter-bar.tsx && git commit -m "feat: add PlanTier/RoiMetrics/ContextWindowUsage types, hooks, TaskFilterBar

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 5: Frontend — Command Center Components

### Task 13: Build `RateLimitGauge`

**Files:**
- Create: `frontend/src/components/analytics/rate-limit-gauge.tsx`

- [ ] **Step 1: Create the component**

```tsx
'use client';

import { useEffect, useState } from 'react';

interface Props {
  label: string;
  used: number;
  limit: number;
  resetAt: string | null; // ISO-8601 timestamp
}

function formatCountdown(resetAt: string): string {
  const diff = new Date(resetAt).getTime() - Date.now();
  if (diff <= 0) return 'now';
  const h = Math.floor(diff / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  const s = Math.floor((diff % 60_000) / 1_000);
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function RateLimitGauge({ label, used, limit, resetAt }: Props) {
  const [countdown, setCountdown] = useState(resetAt ? formatCountdown(resetAt) : null);

  useEffect(() => {
    if (!resetAt) return;
    const id = setInterval(() => setCountdown(formatCountdown(resetAt)), 1_000);
    return () => clearInterval(id);
  }, [resetAt]);

  const pct = limit > 0 ? Math.min((used / limit) * 100, 100) : 0;
  const color = pct < 60 ? 'bg-emerald-500' : pct < 85 ? 'bg-amber-500' : 'bg-red-500';
  const textColor = pct < 60 ? 'text-emerald-400' : pct < 85 ? 'text-amber-400' : 'text-red-400';

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-foreground">{label}</span>
        <span className={textColor}>
          {formatTokens(used)} / {formatTokens(limit)} ({pct.toFixed(1)}%)
        </span>
      </div>
      <div className="h-2 w-full rounded-full bg-muted overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      {countdown && (
        <p className="text-xs text-muted-foreground">
          Resets in <span className="font-mono">{countdown}</span>
        </p>
      )}
    </div>
  );
}
```

---

### Task 14: Build `ContextWindowGauge`

**Files:**
- Create: `frontend/src/components/analytics/context-window-gauge.tsx`

- [ ] **Step 1: Create the component**

```tsx
'use client';

import { useContextUsage } from '@/hooks/use-analytics';

function formatTokens(n: number) {
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

export function ContextWindowGauges() {
  const { data: sessions = [], isLoading } = useContextUsage();

  if (isLoading) {
    return <div className="h-6 animate-pulse bg-muted rounded" />;
  }

  if (sessions.length === 0) {
    return (
      <p className="text-xs text-muted-foreground italic">
        No active sessions — context gauges appear when Claude is running
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {sessions.map((s) => {
        const pct = s.pct_used;
        const color = pct < 60 ? 'bg-emerald-500' : pct < 85 ? 'bg-amber-500' : 'bg-red-500';
        const textColor = pct < 60 ? 'text-emerald-400' : pct < 85 ? 'text-amber-400' : 'text-red-400';
        return (
          <div key={s.session_id} className="space-y-1">
            <div className="flex items-center justify-between text-xs">
              <span className="text-muted-foreground truncate max-w-[60%]">{s.task_title}</span>
              <span className={textColor}>
                {formatTokens(s.tokens_in_window)} / {formatTokens(s.context_limit)} ctx
              </span>
            </div>
            <div className="h-1.5 w-full rounded-full bg-muted overflow-hidden">
              <div
                className={`h-full rounded-full ${color}`}
                style={{ width: `${Math.min(pct, 100)}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}
```

---

### Task 15: Build `CommandCenter`

**Files:**
- Create: `frontend/src/components/analytics/command-center.tsx`

- [ ] **Step 1: Create the component**

```tsx
'use client';

import { useAnalyticsOverview, useBurnRate, useUsageWindows, usePlanTier } from '@/hooks/use-analytics';
import { RateLimitGauge } from './rate-limit-gauge';
import { ContextWindowGauges } from './context-window-gauge';

function Stat({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="space-y-0.5">
      <p className="text-xs text-muted-foreground uppercase tracking-wider">{label}</p>
      <p className="text-2xl font-bold tabular-nums">{value}</p>
      {sub && <p className="text-xs text-muted-foreground">{sub}</p>}
    </div>
  );
}

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function CommandCenter() {
  const { data: overview } = useAnalyticsOverview();
  const { data: burn } = useBurnRate();
  const { data: windows } = useUsageWindows();
  const { data: plan } = usePlanTier();

  const limit5hr = plan?.limit_5hr ?? 19_000;
  const limitWeek = plan?.limit_week ?? 1_000_000;

  const burnLabel = burn
    ? `${formatTokens(Math.round(burn.tokens_per_minute * 60))}/hr — ${
        burn.tokens_per_minute > 0 && windows
          ? `limit in ~${Math.round((limit5hr - (windows.tokens_5hr ?? 0)) / burn.tokens_per_minute / 60)}h`
          : 'at limit pace'
      }`
    : null;

  return (
    <div className="rounded-xl border border-border bg-gradient-to-br from-card to-card/60 p-5 sm:p-6 space-y-6">
      {/* Rate limit gauges */}
      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
        <RateLimitGauge
          label={`5-hour window (${plan?.tier ?? 'pro'} plan)`}
          used={windows?.tokens_5hr ?? 0}
          limit={limit5hr}
          resetAt={windows?.reset_5hr ?? null}
        />
        <RateLimitGauge
          label="Weekly window"
          used={windows?.tokens_week ?? 0}
          limit={limitWeek}
          resetAt={windows?.reset_week ?? null}
        />
      </div>

      {/* Burn rate */}
      {burnLabel && (
        <p className="text-xs text-muted-foreground">
          Burn rate: <span className="font-medium text-foreground">{burnLabel}</span>
        </p>
      )}

      {/* Context gauges */}
      <div className="space-y-2">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Active session context
        </p>
        <ContextWindowGauges />
      </div>

      {/* Headline stats */}
      <div className="grid grid-cols-3 gap-4 pt-2 border-t border-border">
        <Stat
          label="Total Cost"
          value={overview ? `$${overview.estimated_cost_usd.toFixed(2)}` : '—'}
        />
        <Stat
          label="Total Tokens"
          value={overview ? formatTokens(overview.total_input_tokens + overview.total_output_tokens) : '—'}
          sub={overview ? `${formatTokens(overview.total_input_tokens)} in / ${formatTokens(overview.total_output_tokens)} out` : ''}
        />
        <Stat
          label="Sessions"
          value={overview ? String(overview.total_sessions) : '—'}
          sub={overview ? `${overview.active_sessions_today} today` : ''}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 2: TS check**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add frontend/src/components/analytics/ && git commit -m "feat: add CommandCenter, RateLimitGauge, ContextWindowGauge components

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 6: Frontend — ROI Cards + Productivity Section

### Task 16: Build `RoiCards`

**Files:**
- Create: `frontend/src/components/analytics/roi-cards.tsx`

- [ ] **Step 1: Create the component**

```tsx
'use client';

import { useRoiMetrics } from '@/hooks/use-analytics';

function formatCost(v: number | null) {
  if (v === null) return '—';
  if (v < 0.01) return `$${v.toFixed(4)}`;
  return `$${v.toFixed(2)}`;
}

function formatSecs(s: number) {
  if (s < 60) return `${Math.round(s)}s`;
  if (s < 3600) return `${Math.round(s / 60)}m`;
  return `${(s / 3600).toFixed(1)}h`;
}

interface Props { taskId?: string | null }

export function RoiCards({ taskId }: Props) {
  const { data: roi, isLoading } = useRoiMetrics(taskId);

  const cards = [
    {
      label: 'Cost / Commit',
      value: isLoading ? null : formatCost(roi?.cost_per_commit ?? null),
      sub: roi ? `${roi.total_commits} commits` : '',
    },
    {
      label: 'Cost / PR',
      value: isLoading ? null : formatCost(roi?.cost_per_pr ?? null),
      sub: roi ? `${roi.total_prs} PRs` : '',
    },
    {
      label: 'Cost / 100 Lines',
      value: isLoading ? null : (
        roi?.cost_per_loc != null ? `$${(roi.cost_per_loc * 100).toFixed(3)}` : '—'
      ),
      sub: roi ? `${roi.total_loc.toLocaleString()} net lines` : '',
    },
    {
      label: 'Avg Session',
      value: isLoading ? null : (roi ? formatSecs(roi.avg_session_duration_secs) : '—'),
      sub: roi ? `${formatSecs(roi.total_active_time_secs)} active` : '',
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {cards.map((c) => (
        <div key={c.label} className="rounded-xl border border-border bg-card p-4 space-y-1">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">{c.label}</p>
          <p className="text-2xl font-bold">
            {c.value === null
              ? <span className="inline-block w-16 h-7 rounded animate-pulse bg-muted" />
              : c.value}
          </p>
          <p className="text-xs text-muted-foreground">{c.sub}</p>
        </div>
      ))}
    </div>
  );
}
```

---

### Task 17: Build `ProductivitySection`

**Files:**
- Create: `frontend/src/components/analytics/productivity-section.tsx`

- [ ] **Step 1: Create the component**

```tsx
'use client';

import { useRoiMetrics } from '@/hooks/use-analytics';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { TokenEfficiencyChart } from './token-efficiency-chart';

interface Props { taskId?: string | null }

function formatSecs(s: number) {
  if (s < 3600) return `${Math.round(s / 60)}m`;
  return `${(s / 3600).toFixed(1)}h`;
}

export function ProductivitySection({ taskId }: Props) {
  const { data: roi } = useRoiMetrics(taskId);

  const hasOtelData = (roi?.total_commits ?? 0) > 0 || (roi?.total_prs ?? 0) > 0;

  const activityData = roi ? [
    { name: 'Commits', value: roi.total_commits, color: '#6366f1' },
    { name: 'PRs', value: roi.total_prs, color: '#8b5cf6' },
    { name: 'LOC (÷100)', value: Math.round(roi.total_loc / 100), color: '#a78bfa' },
  ] : [];

  return (
    <div className="space-y-6">
      {!hasOtelData && (
        <div className="rounded-xl border border-border bg-card/50 p-4 text-sm text-muted-foreground">
          Commit, PR, and active-time data appears once Claude Code reports OTel metrics.
          Ensure <code className="text-xs bg-muted px-1 py-0.5 rounded">OTEL_EXPORTER_OTLP_ENDPOINT</code> points
          to <code className="text-xs bg-muted px-1 py-0.5 rounded">http://localhost:4318</code>.
        </div>
      )}

      {hasOtelData && (
        <div className="rounded-xl border border-border bg-card p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">Output Activity</h3>
            {roi && (
              <span className="text-xs text-muted-foreground">
                {formatSecs(roi.total_active_time_secs)} active time
              </span>
            )}
          </div>
          <ResponsiveContainer width="100%" height={160}>
            <BarChart data={activityData} barSize={40}>
              <XAxis dataKey="name" tick={{ fontSize: 12 }} />
              <YAxis tick={{ fontSize: 12 }} />
              <Tooltip formatter={(v: number) => [v, '']} />
              <Bar dataKey="value" radius={[4, 4, 0, 0]}>
                {activityData.map((entry) => (
                  <Cell key={entry.name} fill={entry.color} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Token efficiency — reuse existing component */}
      <TokenEfficiencyChart />
    </div>
  );
}
```

- [ ] **Step 2: TS check**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add frontend/src/components/analytics/ && git commit -m "feat: add RoiCards and ProductivitySection components

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 7: Frontend — Page Redesign

### Task 18: Rewrite `analytics/page.tsx`

**Files:**
- Modify: `frontend/src/app/analytics/page.tsx`

- [ ] **Step 1: Rewrite the page**

Next.js 14 requires `useSearchParams()` to be inside a `<Suspense>` boundary. The solution is to split the page into a shell (default export, server-compatible) and an inner client component:

```tsx
import { Suspense } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { AnalyticsPageInner } from '@/components/analytics/analytics-page-inner';

export default function AnalyticsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <div className="border-b border-border px-4 sm:px-6 py-4">
          <h1 className="text-xl font-semibold">Analytics</h1>
          <p className="text-sm text-muted-foreground">
            Claude usage, cost, and productivity intelligence
          </p>
        </div>
        <Suspense fallback={<div className="flex-1 p-6 text-muted-foreground text-sm">Loading…</div>}>
          <AnalyticsPageInner />
        </Suspense>
      </div>
    </div>
  );
}
```

Then create `frontend/src/components/analytics/analytics-page-inner.tsx`:

```tsx
'use client';

import { useState, useEffect } from 'react';
import { useSearchParams, useRouter } from 'next/navigation';
import { CommandCenter } from '@/components/analytics/command-center';
import { TaskFilterBar } from '@/components/analytics/task-filter-bar';
import { RoiCards } from '@/components/analytics/roi-cards';
import { ProductivitySection } from '@/components/analytics/productivity-section';
import { TokenTimeChart } from '@/components/analytics/token-time-chart';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';
import { StageBreakdownChart } from '@/components/analytics/stage-breakdown-chart';
import { CostBreakdownTable } from '@/components/analytics/cost-breakdown-table';
import { SessionTimelineChart } from '@/components/analytics/session-timeline-chart';
import { DevActivityCharts } from '@/components/analytics/dev-activity-charts';
import { TokensByTaskChart } from '@/components/analytics/tokens-by-task-chart';

export function AnalyticsPageInner() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(
    searchParams.get('task')
  );

  // Sync task selection to URL
  useEffect(() => {
    const params = new URLSearchParams(searchParams.toString());
    if (selectedTaskId) {
      params.set('task', selectedTaskId);
    } else {
      params.delete('task');
    }
    router.replace(`?${params.toString()}`, { scroll: false });
  }, [selectedTaskId, searchParams, router]);

  return (
    <main className="flex-1 pb-20 md:pb-6">
          {/* Command Center — always global */}
          <section className="p-4 sm:p-6">
            <CommandCenter />
          </section>

          {/* Sticky task filter */}
          <TaskFilterBar selectedTaskId={selectedTaskId} onSelect={setSelectedTaskId} />

          <div className="p-4 sm:p-6 space-y-10">
            {/* ROI & Cost */}
            <section className="space-y-4">
              <div>
                <h2 className="text-base font-semibold">ROI & Cost</h2>
                <p className="text-sm text-muted-foreground">
                  {selectedTaskId ? 'Filtered to selected task' : 'All tasks combined'}
                </p>
              </div>
              <RoiCards taskId={selectedTaskId} />
              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <CostBreakdownTable />
                <TokensByTaskChart />
              </div>
            </section>

            {/* Usage Trends — global, task as highlight */}
            <section className="space-y-4">
              <h2 className="text-base font-semibold">Usage Trends</h2>
              <TokenTimeChart />
              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <ToolBreakdownChart />
                <LanguageChart />
              </div>
              <StageBreakdownChart />
            </section>

            {/* Productivity */}
            <section className="space-y-4">
              <div>
                <h2 className="text-base font-semibold">Productivity</h2>
                <p className="text-sm text-muted-foreground">
                  Commits, PRs, and lines written (requires OTel)
                </p>
              </div>
              <ProductivitySection taskId={selectedTaskId} />
            </section>

            {/* Session Deep Dive */}
            <section className="space-y-4">
              <div>
                <h2 className="text-base font-semibold">Session Deep Dive</h2>
                <p className="text-sm text-muted-foreground">
                  {selectedTaskId ? 'Sessions for selected task' : 'Select a task above to filter'}
                </p>
              </div>
              <SessionTimelineChart />
              <DevActivityCharts />
            </section>
          </div>
        </main>
  );
}
```

Also add `analytics-page-inner.tsx` to the File Map section under "Frontend — create":

```
| `frontend/src/components/analytics/analytics-page-inner.tsx` | Client component with useSearchParams + full page layout (wrapped in Suspense by page.tsx) |
```

- [ ] **Step 2: TS check**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npx tsc --noEmit 2>&1 | head -30
```

Expected: no errors (fix any import path issues).

- [ ] **Step 3: Build the frontend**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npm run build 2>&1 | tail -20
```

Expected: build succeeds with no errors.

---

### Task 19: Restyle existing components

**Files:**
- Modify: `frontend/src/components/analytics/overview-cards.tsx` and other existing analytics components (className/color token changes only — no prop or hook changes)

- [ ] **Step 1: Update card border radius and spacing to match new design language**

In `overview-cards.tsx` and `cost-breakdown-table.tsx`, update container `className` values:
- `rounded-xl` → already set in most; verify consistency
- Add `bg-card` to any plain divs that are missing it
- Ensure `border border-border` is on all card containers

Check each of these files and make the minimal className fixes needed so they visually match the new `CommandCenter` style:
- `tool-breakdown-chart.tsx` — verify card wrapper has `rounded-xl border border-border bg-card p-5`
- `language-chart.tsx` — same
- `stage-breakdown-chart.tsx` — same
- `token-time-chart.tsx` — same
- `tokens-by-task-chart.tsx` — same
- `session-timeline-chart.tsx` — same
- `dev-activity-charts.tsx` — same

**Rule:** Only change `className` strings. Do not change props, hooks, data, or logic.

- [ ] **Step 2: Final build + TS check**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npx tsc --noEmit 2>&1 | head -10 && npm run build 2>&1 | tail -10
```

Expected: clean build.

- [ ] **Step 3: Run frontend tests**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npm test -- --passWithNoTests 2>&1 | tail -20
```

Expected: all existing tests pass.

- [ ] **Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add frontend/src/ && git commit -m "feat: analytics page full redesign — command center, task filter, ROI sections

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 20: Smoke test the live app

- [ ] **Step 1: Start backend**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo run 2>&1 &
```

- [ ] **Step 2: Start frontend**

```bash
cd /home/utility/Projects/ai-kanban/frontend && npm run dev 2>&1 &
```

- [ ] **Step 3: Verify new endpoints respond**

```bash
curl -s http://localhost:3001/api/analytics/plan-tier | python3 -m json.tool
curl -s http://localhost:3001/api/analytics/roi | python3 -m json.tool
curl -s http://localhost:3001/api/analytics/context-usage | python3 -m json.tool
```

Expected: all three return valid JSON with no 500 errors.

- [ ] **Step 4: Open analytics page in browser**

Navigate to `http://localhost:3000/analytics` (or the configured frontend port).

Verify:
- Command Center hero visible with rate limit gauges
- Task filter bar is sticky
- ROI cards show (with `—` for null values if no OTel data)
- Usage Trends section scrolls correctly
- Productivity section shows OTel placeholder if no data
- Session Deep Dive section present
- No console errors

- [ ] **Step 5: Final commit if any fixes needed**

```bash
cd /home/utility/Projects/ai-kanban && git add -p && git commit -m "fix: analytics page polish after smoke test

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```
