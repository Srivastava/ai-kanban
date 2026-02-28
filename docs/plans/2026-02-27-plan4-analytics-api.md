# Analytics API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `/api/analytics/*` REST endpoints that serve token usage data for all chart dimensions.

**Architecture:** New `analytics` module under `backend/src/api/`. Uses `AnalyticsApiState` (already added in plan3). All aggregations are computed in SQL — no separate pipeline. Route is nested at `/api/analytics` in `routes.rs`.

**Tech Stack:** Rust, Axum, SQLx raw queries with GROUP BY, serde_json, chrono

---

## Context

Key files:
- `backend/src/api/mod.rs` — `AnalyticsApiState` already added in plan3
- `backend/src/api/routes.rs` — add nest for analytics routes
- Pattern: `backend/src/api/logs.rs` — follow this exact pattern for handlers

**Pricing constant:** Claude Sonnet 4.6 is $3/1M input tokens, $15/1M output tokens.

---

## Task 1: Analytics Models

**Files:**
- Create: `backend/src/models/analytics.rs`
- Modify: `backend/src/models/mod.rs`

**Step 1: Create analytics response types**

Create `backend/src/models/analytics.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_sessions: i64,
    pub total_tasks_with_sessions: i64,
    pub estimated_cost_usd: f64,
    pub active_sessions_today: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyTokens {
    pub date: String,          // "2026-02-27"
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct WeeklyTokens {
    pub week_start: String,    // "2026-02-23"
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct MonthlyTokens {
    pub month: String,         // "2026-02"
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct TaskTokens {
    pub task_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionTokens {
    pub session_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub started_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolTokens {
    pub tool_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Serialize)]
pub struct LanguageTokens {
    pub file_ext: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Serialize)]
pub struct EfficiencyRow {
    pub task_id: String,
    pub task_title: String,
    pub total_tokens: i64,
    pub lines_written: i64,
    pub project_loc: i64,
    pub tokens_per_line: Option<f64>,
    pub tokens_per_loc: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SessionTimelineEvent {
    pub sequence_no: i64,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cumulative_total: i64,
    pub timestamp: String,
}
```

**Step 2: Register in models/mod.rs**

Add to `backend/src/models/mod.rs`:

```rust
pub mod analytics;
pub use analytics::*;
```

**Step 3: Verify compilation**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | grep "^error"
```

---

## Task 2: Analytics Repository (SQL queries)

**Files:**
- Create: `backend/src/db/analytics.rs`
- Modify: `backend/src/db/mod.rs`

**Step 1: Create AnalyticsRepository**

Create `backend/src/db/analytics.rs`:

```rust
use crate::models::{
    AnalyticsOverview, DailyTokens, EfficiencyRow, LanguageTokens, MonthlyTokens,
    SessionTimelineEvent, SessionTokens, TaskTokens, ToolTokens, WeeklyTokens,
};
use anyhow::Result;
use sqlx::SqlitePool;

const COST_INPUT_PER_TOKEN: f64 = 3.0 / 1_000_000.0;   // $3 per 1M input
const COST_OUTPUT_PER_TOKEN: f64 = 15.0 / 1_000_000.0; // $15 per 1M output

#[derive(Clone)]
pub struct AnalyticsRepository {
    pool: SqlitePool,
}

impl AnalyticsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn overview(&self) -> Result<AnalyticsOverview> {
        let row = sqlx::query!(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0)  as "total_input!: i64",
                COALESCE(SUM(output_tokens), 0) as "total_output!: i64",
                COUNT(DISTINCT session_id)      as "total_sessions!: i64",
                COUNT(DISTINCT task_id)         as "total_tasks!: i64"
            FROM token_events
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let today_sessions: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM sessions WHERE DATE(started_at) = DATE('now')"
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let input = row.total_input;
        let output = row.total_output;
        let cost = (input as f64 * COST_INPUT_PER_TOKEN)
            + (output as f64 * COST_OUTPUT_PER_TOKEN);

        Ok(AnalyticsOverview {
            total_input_tokens: input,
            total_output_tokens: output,
            total_sessions: row.total_sessions,
            total_tasks_with_sessions: row.total_tasks,
            estimated_cost_usd: (cost * 10000.0).round() / 10000.0,
            active_sessions_today: today_sessions,
        })
    }

    pub async fn daily_tokens(&self, days: i64) -> Result<Vec<DailyTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                DATE(timestamp) as "date!: String",
                COALESCE(SUM(input_tokens), 0)  as "input_tokens!: i64",
                COALESCE(SUM(output_tokens), 0) as "output_tokens!: i64"
            FROM token_events
            WHERE timestamp >= datetime('now', printf('-%d days', ?))
            GROUP BY DATE(timestamp)
            ORDER BY DATE(timestamp) ASC
            "#,
            days
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DailyTokens {
                date: r.date,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
            })
            .collect())
    }

    pub async fn weekly_tokens(&self, weeks: i64) -> Result<Vec<WeeklyTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                DATE(timestamp, 'weekday 0', '-7 days') as "week_start!: String",
                COALESCE(SUM(input_tokens), 0)          as "input_tokens!: i64",
                COALESCE(SUM(output_tokens), 0)         as "output_tokens!: i64"
            FROM token_events
            WHERE timestamp >= datetime('now', printf('-%d days', ?))
            GROUP BY DATE(timestamp, 'weekday 0', '-7 days')
            ORDER BY week_start ASC
            "#,
            weeks * 7
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| WeeklyTokens {
                week_start: r.week_start,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
            })
            .collect())
    }

    pub async fn monthly_tokens(&self, months: i64) -> Result<Vec<MonthlyTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                strftime('%Y-%m', timestamp) as "month!: String",
                COALESCE(SUM(input_tokens), 0)  as "input_tokens!: i64",
                COALESCE(SUM(output_tokens), 0) as "output_tokens!: i64"
            FROM token_events
            WHERE timestamp >= datetime('now', printf('-%d days', ?))
            GROUP BY strftime('%Y-%m', timestamp)
            ORDER BY month ASC
            "#,
            months * 30
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MonthlyTokens {
                month: r.month,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
            })
            .collect())
    }

    pub async fn tokens_by_task(&self) -> Result<Vec<TaskTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                t.id        as "task_id!: String",
                t.title     as "task_title!: String",
                COALESCE(SUM(te.input_tokens), 0)              as "input_tokens!: i64",
                COALESCE(SUM(te.output_tokens), 0)             as "output_tokens!: i64",
                COALESCE(SUM(te.input_tokens + te.output_tokens), 0) as "total_tokens!: i64"
            FROM tasks t
            JOIN token_events te ON te.task_id = t.id
            GROUP BY t.id, t.title
            ORDER BY total_tokens DESC
            LIMIT 50
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| TaskTokens {
                task_id: r.task_id,
                task_title: r.task_title,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
                total_tokens: r.total_tokens,
            })
            .collect())
    }

    pub async fn tokens_by_session(&self) -> Result<Vec<SessionTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                s.id        as "session_id!: String",
                t.title     as "task_title!: String",
                COALESCE(SUM(te.input_tokens), 0)              as "input_tokens!: i64",
                COALESCE(SUM(te.output_tokens), 0)             as "output_tokens!: i64",
                COALESCE(SUM(te.input_tokens + te.output_tokens), 0) as "total_tokens!: i64",
                s.started_at as "started_at: Option<String>"
            FROM sessions s
            JOIN tasks t ON t.id = s.task_id
            JOIN token_events te ON te.session_id = s.id
            GROUP BY s.id, t.title, s.started_at
            ORDER BY total_tokens DESC
            LIMIT 20
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SessionTokens {
                session_id: r.session_id,
                task_title: r.task_title,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
                total_tokens: r.total_tokens,
                started_at: r.started_at,
            })
            .collect())
    }

    pub async fn tokens_by_tool(&self) -> Result<Vec<ToolTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                tool_name as "tool_name!: String",
                COALESCE(SUM(input_tokens), 0)  as "input_tokens!: i64",
                COALESCE(SUM(output_tokens), 0) as "output_tokens!: i64",
                COUNT(*)                         as "call_count!: i64"
            FROM token_events
            WHERE tool_name IS NOT NULL
            GROUP BY tool_name
            ORDER BY (input_tokens + output_tokens) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ToolTokens {
                tool_name: r.tool_name,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
                call_count: r.call_count,
            })
            .collect())
    }

    pub async fn tokens_by_language(&self) -> Result<Vec<LanguageTokens>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                file_ext as "file_ext!: String",
                COALESCE(SUM(input_tokens), 0)  as "input_tokens!: i64",
                COALESCE(SUM(output_tokens), 0) as "output_tokens!: i64",
                COUNT(*)                         as "call_count!: i64"
            FROM token_events
            WHERE file_ext IS NOT NULL
            GROUP BY file_ext
            ORDER BY (input_tokens + output_tokens) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| LanguageTokens {
                file_ext: r.file_ext,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
                call_count: r.call_count,
            })
            .collect())
    }

    pub async fn token_efficiency(&self) -> Result<Vec<EfficiencyRow>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                t.id    as "task_id!: String",
                t.title as "task_title!: String",
                COALESCE(SUM(te.input_tokens + te.output_tokens), 0) as "total_tokens!: i64",
                COALESCE(sm.lines_written, 0) as "lines_written!: i64",
                COALESCE(sm.project_loc, 0)   as "project_loc!: i64"
            FROM tasks t
            JOIN token_events te ON te.task_id = t.id
            LEFT JOIN session_metrics sm ON sm.session_id = (
                SELECT id FROM sessions WHERE task_id = t.id
                ORDER BY started_at DESC LIMIT 1
            )
            GROUP BY t.id, t.title, sm.lines_written, sm.project_loc
            ORDER BY total_tokens DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let tokens_per_line = if r.lines_written > 0 {
                    Some(r.total_tokens as f64 / r.lines_written as f64)
                } else {
                    None
                };
                let tokens_per_loc = if r.project_loc > 0 {
                    Some(r.total_tokens as f64 / r.project_loc as f64)
                } else {
                    None
                };
                EfficiencyRow {
                    task_id: r.task_id,
                    task_title: r.task_title,
                    total_tokens: r.total_tokens,
                    lines_written: r.lines_written,
                    project_loc: r.project_loc,
                    tokens_per_line,
                    tokens_per_loc,
                }
            })
            .collect())
    }

    pub async fn session_timeline(&self, session_id: &str) -> Result<Vec<SessionTimelineEvent>> {
        // Compute cumulative sum in Rust (SQLite window functions need version 3.25+)
        let rows = sqlx::query!(
            r#"
            SELECT
                COALESCE(sequence_no, 0) as "sequence_no!: i64",
                event_type               as "event_type!: String",
                tool_name                as "tool_name: Option<String>",
                input_tokens             as "input_tokens!: i64",
                output_tokens            as "output_tokens!: i64",
                timestamp                as "timestamp!: String"
            FROM token_events
            WHERE session_id = ?
            ORDER BY COALESCE(sequence_no, 0) ASC, id ASC
            "#,
            session_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut cumulative: i64 = 0;
        Ok(rows
            .into_iter()
            .map(|r| {
                cumulative += r.input_tokens + r.output_tokens;
                SessionTimelineEvent {
                    sequence_no: r.sequence_no,
                    event_type: r.event_type,
                    tool_name: r.tool_name,
                    input_tokens: r.input_tokens,
                    output_tokens: r.output_tokens,
                    cumulative_total: cumulative,
                    timestamp: r.timestamp,
                }
            })
            .collect())
    }
}
```

**Step 2: Register in db/mod.rs**

Add to `backend/src/db/mod.rs`:

```rust
mod analytics;
pub use analytics::AnalyticsRepository;
```

**Step 3: Verify compilation**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | grep "^error"
```

---

## Task 3: Analytics API Handlers

**Files:**
- Create: `backend/src/api/analytics.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/api/routes.rs`
- Modify: `backend/src/main.rs`

**Step 1: Create the handler file**

Create `backend/src/api/analytics.rs`:

```rust
use super::AnalyticsApiState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tracing::{error, instrument};

#[derive(Deserialize)]
struct DaysQuery {
    days: Option<i64>,
}

#[derive(Deserialize)]
struct WeeksQuery {
    weeks: Option<i64>,
}

#[derive(Deserialize)]
struct MonthsQuery {
    months: Option<i64>,
}

pub fn analytics_routes() -> Router<AnalyticsApiState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/tokens/daily", get(daily_tokens))
        .route("/tokens/weekly", get(weekly_tokens))
        .route("/tokens/monthly", get(monthly_tokens))
        .route("/tokens/by-task", get(tokens_by_task))
        .route("/tokens/by-session", get(tokens_by_session))
        .route("/tokens/by-tool", get(tokens_by_tool))
        .route("/tokens/by-language", get(tokens_by_language))
        .route("/tokens/efficiency", get(token_efficiency))
        .route("/sessions/:id/timeline", get(session_timeline))
}

#[instrument(skip(state))]
async fn overview(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    match state.analytics.overview().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get analytics overview");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn daily_tokens(
    State(state): State<AnalyticsApiState>,
    Query(q): Query<DaysQuery>,
) -> impl IntoResponse {
    let days = q.days.unwrap_or(30).min(365);
    match state.analytics.daily_tokens(days).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get daily tokens");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn weekly_tokens(
    State(state): State<AnalyticsApiState>,
    Query(q): Query<WeeksQuery>,
) -> impl IntoResponse {
    let weeks = q.weeks.unwrap_or(12).min(52);
    match state.analytics.weekly_tokens(weeks).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get weekly tokens");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn monthly_tokens(
    State(state): State<AnalyticsApiState>,
    Query(q): Query<MonthsQuery>,
) -> impl IntoResponse {
    let months = q.months.unwrap_or(6).min(24);
    match state.analytics.monthly_tokens(months).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get monthly tokens");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_task(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    match state.analytics.tokens_by_task().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get tokens by task");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_session(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    match state.analytics.tokens_by_session().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get tokens by session");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_tool(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    match state.analytics.tokens_by_tool().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get tokens by tool");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_language(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    match state.analytics.tokens_by_language().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get tokens by language");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn token_efficiency(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    match state.analytics.token_efficiency().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get token efficiency");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

#[instrument(skip(state), fields(session_id = %session_id))]
async fn session_timeline(
    State(state): State<AnalyticsApiState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.analytics.session_timeline(&session_id).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(error = %e, "Failed to get session timeline");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}
```

**Step 2: Update AnalyticsApiState in api/mod.rs**

The `AnalyticsApiState` (added in plan3) needs an `analytics` field. Update it:

```rust
use crate::db::AnalyticsRepository;

#[derive(Clone)]
pub struct AnalyticsApiState {
    pub analytics: AnalyticsRepository,
}

impl From<AppState> for AnalyticsApiState {
    fn from(state: AppState) -> Self {
        AnalyticsApiState {
            analytics: state.analytics,
        }
    }
}
```

Also add `analytics: AnalyticsRepository` to `AppState` and its `new()` method.

**Step 3: Register analytics module in api/mod.rs**

Add at bottom of module declarations:

```rust
mod analytics;
pub use analytics::analytics_routes;
```

**Step 4: Add route in routes.rs**

Open `backend/src/api/routes.rs`. Add:

```rust
use crate::api::analytics::analytics_routes;

// In create_router():
let analytics_state: AnalyticsApiState = state.clone().into();

Router::new()
    // ... existing routes ...
    .nest("/api/analytics", analytics_routes().with_state(analytics_state))
```

**Step 5: Update main.rs**

Add `AnalyticsRepository::new(pool.clone())` and pass to `AppState::new()`.

**Step 6: Verify compilation and test**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | grep "^error"
```

```bash
# Start server and test endpoints
cargo run &
sleep 2
curl -s http://localhost:3001/api/analytics/overview | python3 -m json.tool
curl -s "http://localhost:3001/api/analytics/tokens/daily?days=30" | python3 -m json.tool
curl -s http://localhost:3001/api/analytics/tokens/by-tool | python3 -m json.tool
kill %1
```

Expected: valid JSON responses (empty arrays/zero counts since no data yet).

**Step 7: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add backend/src/models/analytics.rs \
        backend/src/models/mod.rs \
        backend/src/db/analytics.rs \
        backend/src/db/mod.rs \
        backend/src/api/analytics.rs \
        backend/src/api/mod.rs \
        backend/src/api/routes.rs \
        backend/src/main.rs
git commit -m "feat(api): add /api/analytics/* endpoints

- /overview: total tokens, cost estimate, sessions today
- /tokens/daily|weekly|monthly: time-series data
- /tokens/by-task|by-session|by-tool|by-language: breakdown charts
- /tokens/efficiency: tokens-per-line-written and tokens-per-LOC
- /sessions/:id/timeline: per-JSONL-event cumulative token chart"
```
