use crate::db::{AnalyticsRepository, OtelMetricsRepository};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tracing::{debug, error, info, instrument};

use super::AppState;

#[derive(Clone)]
pub struct AnalyticsApiState {
    pub analytics: AnalyticsRepository,
    pub otel_repo: OtelMetricsRepository,
    pub usage_cache: crate::api::claude_usage_cli::SharedUsageCache,
}

impl From<AppState> for AnalyticsApiState {
    fn from(state: AppState) -> Self {
        let pool = state.token_events.pool().clone();
        AnalyticsApiState {
            analytics: AnalyticsRepository::new(pool.clone()),
            otel_repo: OtelMetricsRepository::new(pool),
            usage_cache: crate::api::claude_usage_cli::start_usage_daemon(state.queue),
        }
    }
}

#[derive(Deserialize, Debug)]
struct DaysQuery {
    #[serde(default = "default_days")]
    days: i64,
    task_id: Option<String>,
}

fn default_days() -> i64 {
    30
}

#[derive(Deserialize, Debug)]
struct WeeksQuery {
    #[serde(default = "default_weeks")]
    weeks: i64,
    task_id: Option<String>,
}

fn default_weeks() -> i64 {
    12
}

#[derive(Deserialize, Debug)]
struct MonthsQuery {
    #[serde(default = "default_months")]
    months: i64,
    task_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TaskFilterQuery {
    task_id: Option<String>,
}

fn default_months() -> i64 {
    12
}

pub fn analytics_routes() -> Router<AnalyticsApiState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/usage-windows", get(usage_windows))
        .route("/tokens/daily", get(daily_tokens))
        .route("/tokens/weekly", get(weekly_tokens))
        .route("/tokens/monthly", get(monthly_tokens))
        .route("/tokens/by-task", get(tokens_by_task))
        .route("/tokens/by-session", get(tokens_by_session))
        .route("/tokens/by-tool", get(tokens_by_tool))
        .route("/tokens/by-language", get(tokens_by_language))
        .route("/tokens/efficiency", get(token_efficiency))
        .route("/tokens/by-stage", get(tokens_by_stage))
        // literal route BEFORE param route to avoid "summary" being treated as session ID
        .route("/sessions/summary", get(session_summary))
        .route("/sessions/:id/timeline", get(session_timeline))
        .route("/tasks/:id/task-timeline", get(task_timeline))
        .route("/cost/by-task", get(cost_by_task))
        .route("/burn-rate", get(burn_rate))
        .route("/dev-activity", get(dev_activity))
        .route("/plan-tier", get(plan_tier_handler))
        .route("/roi", get(roi_metrics_handler))
        .route("/context-usage", get(context_usage_handler))
}

#[instrument(skip(state))]
async fn overview(
    State(state): State<AnalyticsApiState>,
) -> impl IntoResponse {
    info!("API: Getting analytics overview");
    match state.analytics.overview().await {
        Ok(overview) => {
            debug!("API: Analytics overview retrieved");
            Json(overview).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get analytics overview");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

async fn usage_windows(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    info!("API: Getting usage windows from daemon cache");
    let plan = crate::api::plan_tier::plan_tier_from_env();

    let cache_guard = state.usage_cache.read();
    let (cli, last_poll_no_data, has_prior_data) = match cache_guard {
        Ok(c) => (c.data.clone(), c.last_poll_no_data, c.fetched_at.is_some()),
        Err(_) => (crate::api::claude_usage_cli::ClaudeCliUsage::default(), false, false),
    };

    // no_data = true only when: last poll failed AND we had a prior successful poll
    // (if daemon never had data, JSONL fallback serves the response — no_data stays false)
    let no_data = last_poll_no_data && has_prior_data;

    let (tokens_5hr, tokens_week, reset_5hr, reset_week) =
        if cli.pct_5hr.is_some() || cli.pct_week.is_some() {
            let t5 = cli.pct_5hr
                .map(|p| ((p / 100.0) * plan.limit_5hr as f64).round() as i64)
                .unwrap_or(0);
            let tw = cli.pct_week
                .map(|p| ((p / 100.0) * plan.limit_week as f64).round() as i64)
                .unwrap_or(0);
            let r5 = cli.reset_5hr;
            let rw = cli.reset_week.unwrap_or_else(|| {
                let j = crate::api::claude_jsonl::read_claude_usage();
                crate::api::claude_jsonl::reset_week_from_earliest(j.earliest_week)
            });
            (t5, tw, r5, Some(rw))
        } else {
            // Daemon hasn't gotten data yet — use JSONL
            let j = crate::api::claude_jsonl::read_claude_usage();
            let r5 = crate::api::claude_jsonl::reset_5hr_from_earliest(j.earliest_5hr);
            let rw = crate::api::claude_jsonl::reset_week_from_earliest(j.earliest_week);
            (j.tokens_5hr, j.tokens_week, r5, Some(rw))
        };

    let windows = crate::models::UsageWindows {
        tokens_5hr,
        tokens_week,
        limit_5hr: plan.limit_5hr,
        limit_week: plan.limit_week,
        reset_5hr,
        reset_week,
        no_data,
    };
    Json(windows).into_response()
}

#[instrument(skip(state))]
async fn daily_tokens(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<DaysQuery>,
) -> impl IntoResponse {
    info!(days = query.days, "API: Getting daily tokens");
    match state.analytics.daily_tokens(query.days, query.task_id.as_deref()).await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Daily tokens retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get daily tokens");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn weekly_tokens(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<WeeksQuery>,
) -> impl IntoResponse {
    info!(weeks = query.weeks, "API: Getting weekly tokens");
    match state.analytics.weekly_tokens(query.weeks, query.task_id.as_deref()).await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Weekly tokens retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get weekly tokens");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn monthly_tokens(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<MonthsQuery>,
) -> impl IntoResponse {
    info!(months = query.months, "API: Getting monthly tokens");
    match state.analytics.monthly_tokens(query.months, query.task_id.as_deref()).await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Monthly tokens retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get monthly tokens");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_task(
    State(state): State<AnalyticsApiState>,
) -> impl IntoResponse {
    info!("API: Getting tokens by task");
    match state.analytics.tokens_by_task().await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Tokens by task retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get tokens by task");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_session(
    State(state): State<AnalyticsApiState>,
) -> impl IntoResponse {
    info!("API: Getting tokens by session");
    match state.analytics.tokens_by_session().await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Tokens by session retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get tokens by session");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_tool(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<TaskFilterQuery>,
) -> impl IntoResponse {
    info!("API: Getting tokens by tool");
    match state.analytics.tokens_by_tool(query.task_id.as_deref()).await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Tokens by tool retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get tokens by tool");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn tokens_by_language(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<TaskFilterQuery>,
) -> impl IntoResponse {
    info!("API: Getting tokens by language");
    match state.analytics.tokens_by_language(query.task_id.as_deref()).await {
        Ok(tokens) => {
            debug!(count = tokens.len(), "API: Tokens by language retrieved");
            Json(tokens).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get tokens by language");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn token_efficiency(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<TaskFilterQuery>,
) -> impl IntoResponse {
    info!("API: Getting token efficiency");
    match state.analytics.token_efficiency(query.task_id.as_deref()).await {
        Ok(efficiency) => {
            debug!(count = efficiency.len(), "API: Token efficiency retrieved");
            Json(efficiency).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get token efficiency");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn session_timeline(
    State(state): State<AnalyticsApiState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    info!(session_id = %session_id, "API: Getting session timeline");
    match state.analytics.session_timeline(&session_id).await {
        Ok(timeline) => {
            debug!(count = timeline.len(), "API: Session timeline retrieved");
            Json(timeline).into_response()
        }
        Err(e) => {
            error!(session_id = %session_id, error = %e, "API: Failed to get session timeline");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn cost_by_task(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    info!("API: Getting cost by task");
    match state.analytics.cost_by_task().await {
        Ok(data) => { debug!(count = data.len(), "retrieved"); Json(data).into_response() }
        Err(e) => { error!(error = %e, "failed"); (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response() }
    }
}

#[instrument(skip(state))]
async fn tokens_by_stage(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<TaskFilterQuery>,
) -> impl IntoResponse {
    info!("API: Getting tokens by stage");
    match state.analytics.tokens_by_stage(query.task_id.as_deref()).await {
        Ok(data) => { debug!(count = data.len(), "retrieved"); Json(data).into_response() }
        Err(e) => { error!(error = %e, "failed"); (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response() }
    }
}

#[instrument(skip(state))]
async fn session_summary(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    info!("API: Getting session summary");
    match state.analytics.session_summary().await {
        Ok(data) => { debug!("retrieved"); Json(data).into_response() }
        Err(e) => { error!(error = %e, "failed"); (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response() }
    }
}

#[instrument(skip(state))]
async fn burn_rate(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    info!("API: Getting burn rate");
    match state.analytics.burn_rate().await {
        Ok(data) => { debug!("retrieved"); Json(data).into_response() }
        Err(e) => { error!(error = %e, "failed"); (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response() }
    }
}

#[instrument(skip(state))]
async fn task_timeline(
    State(state): State<AnalyticsApiState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %task_id, "API: Getting task timeline");
    match state.analytics.task_timeline(&task_id).await {
        Ok(data) => { debug!(count = data.len(), "retrieved"); Json(data).into_response() }
        Err(e) => { error!(error = %e, "failed"); (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response() }
    }
}

#[derive(Deserialize, Debug)]
struct TaskIdQuery {
    task_id: Option<String>,
}

#[instrument(skip(state))]
pub async fn dev_activity(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<TaskIdQuery>,
) -> impl IntoResponse {
    info!(task_id = ?query.task_id, "API: Getting dev activity");
    match state.otel_repo.dev_activity(query.task_id.as_deref()).await {
        Ok(rows) => {
            debug!(count = rows.len(), "API: Dev activity retrieved");
            Json(rows).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get dev activity");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

async fn plan_tier_handler() -> impl IntoResponse {
    info!("API: Getting plan tier");
    Json(crate::api::plan_tier::plan_tier_from_env()).into_response()
}

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
