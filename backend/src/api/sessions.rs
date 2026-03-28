use super::SessionApiState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize)]
struct QueueInfo {
    position: usize,
    task_id: String,
    task_title: String,
    stage: String,
    queued_at: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct StartSessionRequest {
    task_id: String,
    stage: String,
}

pub fn session_routes() -> Router<SessionApiState> {
    Router::new()
        .route("/", get(list_sessions))
        .route("/all", get(list_all_sessions))
        .route("/queue", get(get_queue))
        // literal prefix route before param route to avoid "by-claude-id" matching /:id
        .route("/by-claude-id/:id", get(get_by_claude_id))
        .route("/:id/stop", post(stop_session))
        .route("/:id", get(get_session))
}

#[derive(Debug, Deserialize)]
struct SessionListQuery {
    status: Option<String>, // comma-separated: "pending,failed"
    limit: Option<i64>,
}

/// GET /api/sessions/all?status=failed,pending&limit=50
/// Returns sessions across all tasks, optionally filtered by status.
#[instrument(skip(state))]
async fn list_all_sessions(
    State(state): State<SessionApiState>,
    Query(q): Query<SessionListQuery>,
) -> impl IntoResponse {
    let statuses: Vec<&str> = q
        .status
        .as_deref()
        .map(|s| s.split(',').map(|x| x.trim()).collect())
        .unwrap_or_default();

    match state
        .session_repo
        .list_recent(statuses.as_slice(), q.limit.unwrap_or(100))
        .await
    {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn list_sessions(State(state): State<SessionApiState>) -> impl IntoResponse {
    let queued = state.queue.get_queued_tasks().await;
    let active_count = state.queue.active_count().await;
    let queue_info: Vec<QueueInfo> = queued
        .into_iter()
        .enumerate()
        .map(|(i, qt)| QueueInfo {
            position: i,
            task_id: qt.task.id,
            task_title: qt.task.title,
            stage: qt.stage,
            queued_at: qt.queued_at.to_rfc3339(),
        })
        .collect();

    Json(serde_json::json!({
        "active_count": active_count,
        "queued": queue_info
    }))
    .into_response()
}

#[instrument(skip(state))]
async fn get_session(
    State(state): State<SessionApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.session_repo.find(&id).await {
        Ok(session) => Json(session).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn stop_session(
    State(state): State<SessionApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.queue.stop_session(&id).await {
        Ok(()) => {
            // Trigger next task in queue
            let _ = state.queue.on_session_complete(&id).await;
            Json(serde_json::json!({ "status": "stopped" })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn get_by_claude_id(
    State(state): State<SessionApiState>,
    Path(claude_id): Path<String>,
) -> impl IntoResponse {
    match state
        .session_repo
        .find_by_claude_session_id(&claude_id)
        .await
    {
        Ok(Some(session)) => Json(session).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[instrument(skip(state))]
async fn get_queue(State(state): State<SessionApiState>) -> impl IntoResponse {
    let queued = state.queue.get_queued_tasks().await;
    let queue_info: Vec<QueueInfo> = queued
        .into_iter()
        .enumerate()
        .map(|(i, qt)| QueueInfo {
            position: i,
            task_id: qt.task.id,
            task_title: qt.task.title,
            stage: qt.stage,
            queued_at: qt.queued_at.to_rfc3339(),
        })
        .collect();

    Json(queue_info)
}
