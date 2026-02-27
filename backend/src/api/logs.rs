use crate::api::AppState;
use crate::models::{CreateLog, LogFilter};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tracing::{debug, error, info, instrument, warn};

#[derive(Clone)]
pub struct LogApiState {
    pub repo: crate::db::LogRepository,
}

#[derive(Deserialize, Debug)]
struct LogQuery {
    level: Option<String>,
    source: Option<String>,
    task_id: Option<String>,
    session_id: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
}

pub fn log_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_logs).post(create_log))
}

#[instrument(skip(state))]
async fn list_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> impl IntoResponse {
    debug!(
        level = ?query.level,
        source = ?query.source,
        task_id = ?query.task_id,
        session_id = ?query.session_id,
        "API: Listing logs"
    );

    let filter = LogFilter {
        level: query.level,
        source: query.source,
        task_id: query.task_id,
        session_id: query.session_id,
        limit: query.limit,
        offset: query.offset,
    };

    match state.logs.list(filter).await {
        Ok(logs) => {
            debug!(count = logs.len(), "API: Logs retrieved");
            Json(logs).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to list logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn create_log(
    State(state): State<AppState>,
    Json(create): Json<CreateLog>,
) -> impl IntoResponse {
    info!(
        level = %create.level,
        source = ?create.source,
        task_id = ?create.task_id,
        "API: Creating log entry"
    );

    // Validate level
    let level = create.level.to_uppercase();
    if !["DEBUG", "INFO", "WARN", "ERROR"].contains(&level.as_str()) {
        warn!(invalid_level = %create.level, "API: Invalid log level");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid log level. Must be DEBUG, INFO, WARN, or ERROR" })),
        ).into_response();
    }

    let mut create = create;
    create.level = level;
    create.source = Some(create.source.unwrap_or_else(|| "frontend".to_string()));

    match state.logs.create(create).await {
        Ok(log) => {
            info!(log_id = %log.id, "API: Log entry created");
            (StatusCode::CREATED, Json(log)).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to create log entry");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
