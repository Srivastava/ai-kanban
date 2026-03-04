use crate::db::{AnalyticsRepository, OtelMetricsRepository};
use crate::models::DevActivityRow;
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
}

impl From<AppState> for AnalyticsApiState {
    fn from(state: AppState) -> Self {
        let pool = state.token_events.pool().clone();
        AnalyticsApiState {
            analytics: AnalyticsRepository::new(pool.clone()),
            otel_repo: OtelMetricsRepository::new(pool),
        }
    }
}

#[derive(Deserialize, Debug)]
struct DaysQuery {
    #[serde(default = "default_days")]
    days: i64,
}

fn default_days() -> i64 {
    30
}

#[derive(Deserialize, Debug)]
struct WeeksQuery {
    #[serde(default = "default_weeks")]
    weeks: i64,
}

fn default_weeks() -> i64 {
    12
}

#[derive(Deserialize, Debug)]
struct MonthsQuery {
    #[serde(default = "default_months")]
    months: i64,
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
        .route("/cost/by-task", get(cost_by_task))
        .route("/burn-rate", get(burn_rate))
        .route("/dev-activity", get(dev_activity))
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

#[instrument(skip(state))]
async fn usage_windows(
    State(state): State<AnalyticsApiState>,
) -> impl IntoResponse {
    info!("API: Getting usage windows");
    let limit_5hr: i64 = std::env::var("CLAUDE_5HR_TOKEN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let limit_week: i64 = std::env::var("CLAUDE_WEEKLY_TOKEN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    match state.analytics.usage_windows(limit_5hr, limit_week).await {
        Ok(windows) => {
            debug!("API: Usage windows retrieved");
            Json(windows).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to get usage windows");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn daily_tokens(
    State(state): State<AnalyticsApiState>,
    Query(query): Query<DaysQuery>,
) -> impl IntoResponse {
    info!(days = query.days, "API: Getting daily tokens");
    match state.analytics.daily_tokens(query.days).await {
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
    match state.analytics.weekly_tokens(query.weeks).await {
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
    match state.analytics.monthly_tokens(query.months).await {
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
) -> impl IntoResponse {
    info!("API: Getting tokens by tool");
    match state.analytics.tokens_by_tool().await {
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
) -> impl IntoResponse {
    info!("API: Getting tokens by language");
    match state.analytics.tokens_by_language().await {
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
) -> impl IntoResponse {
    info!("API: Getting token efficiency");
    match state.analytics.token_efficiency().await {
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
async fn tokens_by_stage(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    info!("API: Getting tokens by stage");
    match state.analytics.tokens_by_stage().await {
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
pub async fn dev_activity(
    State(state): State<AnalyticsApiState>,
) -> impl IntoResponse {
    info!("API: Getting dev activity");
    match state.otel_repo.dev_activity().await {
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
