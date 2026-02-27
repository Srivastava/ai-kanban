use super::SessionApiState;
use axum::{
    extract::{Path, State},
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
        .route("/queue", get(get_queue))
        .route("/:id/stop", post(stop_session))
        .route("/:id", get(get_session))
}

#[instrument(skip(state))]
async fn list_sessions(
    State(state): State<SessionApiState>,
) -> impl IntoResponse {
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
    })).into_response()
}

#[instrument(skip(state))]
async fn get_session(
    State(state): State<SessionApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.queue.is_session_active(&id).await {
        Json(serde_json::json!({
            "id": id,
            "status": "running"
        })).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        ).into_response()
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
        ).into_response(),
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
