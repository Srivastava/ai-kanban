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
        .route("/:id/sessions/continue", post(continue_session))
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
    // Remove from pending queue and stop any running Claude process for this task
    if let Some(queue) = &state.queue {
        // Remove from pending queue first
        let _ = queue.dequeue(&id).await;
        // Then stop any running session
        if let Some(session_id) = queue.get_active_session_for_task(&id).await {
            let _ = queue.stop_session(&session_id).await;
        }
    }
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
    match queue.enqueue(task, stage, None).await {
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
async fn continue_session(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "API: Continuing Claude session with comment history");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Queue not available" }))).into_response(),
    };

    // Build conversation context from comment thread
    let comments = state.comment_repo.list_for_task(&id).await.unwrap_or_default();
    let conversation_context = if comments.is_empty() {
        None
    } else {
        let history = comments.iter().flat_map(|c| {
            let prefix = if c.comment.author == "claude" { "[Claude]" } else { "[You]" };
            let mut lines = vec![format!("{}: {}", prefix, c.comment.content)];
            for reply in &c.replies {
                let rprefix = if reply.author == "claude" { "[Claude]" } else { "[You]" };
                lines.push(format!("  {}: {}", rprefix, reply.content));
            }
            lines
        }).collect::<Vec<_>>().join("\n");
        Some(history)
    };

    let stage = task.stage.clone();
    match queue.enqueue(task, stage, conversation_context).await {
        Ok(()) => Json(serde_json::json!({ "status": "queued" })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
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
            // Auto-stop session when task moves to Done
            if body.stage == "done" {
                if let Some(queue) = &state.queue {
                    if let Some(session_id) = queue.get_active_session_for_task(&id).await {
                        let _ = queue.stop_session(&session_id).await;
                    }
                }
            }
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
