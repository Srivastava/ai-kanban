use super::TaskApiState;
use crate::models::{CreateTask, UpdateTask};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tracing::{debug, error, info, instrument};

#[derive(Deserialize, Debug)]
struct ListQuery {
    stage: Option<String>,
}

#[derive(Deserialize, Debug)]
struct MoveRequest {
    stage: String,
}

pub fn task_routes() -> Router<TaskApiState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).patch(update_task).delete(delete_task))
        .route("/:id/move", post(move_task))
        .route("/:id/sessions", post(start_session))
}

#[instrument(skip(state))]
async fn list_tasks(
    State(state): State<TaskApiState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    debug!(stage = ?query.stage, "API: Listing tasks");
    match state.repo.list(query.stage.as_deref()).await {
        Ok(tasks) => {
            debug!(count = tasks.len(), "API: Tasks retrieved");
            Json(tasks).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to list tasks");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn get_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    debug!(task_id = %id, "API: Getting task");
    match state.repo.find(&id).await {
        Ok(task) => {
            debug!(task_id = %id, "API: Task retrieved");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "API: Task not found");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn create_task(
    State(state): State<TaskApiState>,
    Json(create): Json<CreateTask>,
) -> impl IntoResponse {
    info!(title = %create.title, project_path = %create.project_path, "API: Creating task");
    match state.repo.create(create).await {
        Ok(task) => {
            info!(task_id = %task.id, "API: Task created");
            (StatusCode::CREATED, Json(task)).into_response()
        }
        Err(e) => {
            error!(error = %e, "API: Failed to create task");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn update_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
    Json(update): Json<UpdateTask>,
) -> impl IntoResponse {
    info!(task_id = %id, "API: Updating task");
    match state.repo.update(&id, update).await {
        Ok(task) => {
            info!(task_id = %id, new_stage = %task.stage, "API: Task updated");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "API: Failed to update task");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn delete_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "API: Deleting task");
    match state.repo.delete(&id).await {
        Ok(()) => {
            info!(task_id = %id, "API: Task deleted");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "API: Failed to delete task");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn start_session(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "API: Starting Claude session for task");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => {
            error!(task_id = %id, error = %e, "API: Task not found for session start");
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            ).into_response();
        }
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => {
            error!("API: Session queue not initialized");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Session queue not available" })),
            ).into_response();
        }
    };

    let stage = task.stage.clone();
    match queue.enqueue(task, stage).await {
        Ok(()) => {
            info!(task_id = %id, "API: Task enqueued for Claude session");
            Json(serde_json::json!({ "status": "queued" })).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "API: Failed to enqueue task");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            ).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn move_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
    Json(body): Json<MoveRequest>,
) -> impl IntoResponse {
    info!(task_id = %id, to_stage = %body.stage, "API: Moving task");
    match state.repo.move_to_stage(&id, &body.stage).await {
        Ok(task) => {
            info!(task_id = %id, new_stage = %task.stage, "API: Task moved");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, to_stage = %body.stage, error = %e, "API: Failed to move task");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
