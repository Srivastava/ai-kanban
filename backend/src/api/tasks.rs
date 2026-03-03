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
    debug!(stage_filter = ?query.stage, "Listing tasks");
    match state.repo.list(query.stage.as_deref()).await {
        Ok(tasks) => {
            debug!(count = tasks.len(), stage_filter = ?query.stage, "Tasks retrieved");
            Json(tasks).into_response()
        }
        Err(e) => {
            error!(stage_filter = ?query.stage, error = %e, "Failed to list tasks");
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
    debug!(task_id = %id, "Getting task");
    match state.repo.find(&id).await {
        Ok(task) => {
            debug!(task_id = %id, title = %task.title, stage = %task.stage, "Task retrieved");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Task not found");
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
    info!(title = %create.title, project_path = %create.project_path, "Creating task");
    match state.repo.create(create).await {
        Ok(task) => {
            info!(task_id = %task.id, title = %task.title, stage = %task.stage, project_path = %task.project_path, "Task created");
            (StatusCode::CREATED, Json(task)).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to create task");
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
    info!(
        task_id = %id,
        update_title = ?update.title,
        update_stage = ?update.stage,
        update_context = update.context.is_some(),
        update_description = update.description.is_some(),
        "Updating task"
    );
    match state.repo.update(&id, update).await {
        Ok(task) => {
            info!(task_id = %id, title = %task.title, stage = %task.stage, "Task updated");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to update task");
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
    info!(task_id = %id, "Deleting task");
    // Remove from pending queue and stop any running Claude process for this task
    if let Some(queue) = &state.queue {
        let dequeued = queue.dequeue(&id).await;
        if dequeued {
            info!(task_id = %id, "Removed task from pending queue before delete");
        }
        if let Some(session_id) = queue.get_active_session_for_task(&id).await {
            info!(task_id = %id, session_id = %session_id, "Stopping active session before task delete");
            let _ = queue.stop_session(&session_id).await;
        }
    }
    match state.repo.delete(&id).await {
        Ok(()) => {
            info!(task_id = %id, "Task deleted");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to delete task");
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
    info!(task_id = %id, "Starting Claude session for task");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => {
            error!(task_id = %id, error = %e, "Task not found for session start");
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            ).into_response();
        }
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => {
            error!(task_id = %id, "Session queue not initialized");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Session queue not available" })),
            ).into_response();
        }
    };

    let stage = task.stage.clone();
    info!(task_id = %id, title = %task.title, stage = %stage, "Enqueuing task for Claude session");
    match queue.enqueue(task, stage, None, None).await {
        Ok(()) => {
            info!(task_id = %id, "Task enqueued for Claude session");
            Json(serde_json::json!({ "status": "queued" })).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to enqueue task for Claude session");
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
    info!(task_id = %id, "Continuing Claude session with comment history");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => {
            error!(task_id = %id, error = %e, "Task not found for continue session");
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e.to_string() }))).into_response();
        }
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => {
            error!(task_id = %id, "Session queue not available for continue session");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Queue not available" }))).into_response();
        }
    };

    // Build conversation context from comment thread
    let comments = state.comment_repo.list_for_task(&id).await.unwrap_or_default();
    let total_comments = comments.len();
    let total_replies: usize = comments.iter().map(|c| c.replies.len()).sum();
    let conversation_context = if comments.is_empty() {
        None
    } else {
        let history = comments.iter().flat_map(|c| {
            let prefix = if c.comment.author == "claude" { "[Claude]" } else { "[You]" };
            let mut lines = vec![format!("{}: {}", prefix, c.comment.content)];
            for reply in &c.replies {
                let reply_prefix = if reply.author == "claude" { "[Claude]" } else { "[You]" };
                lines.push(format!("  {}: {}", reply_prefix, reply.content));
            }
            lines
        }).collect::<Vec<_>>().join("\n");
        Some(history)
    };

    // Look up claude_session_id from the prior session for true resume
    let resume_claude_session_id = if let Some(ref prior_session_id) = task.session_id {
        match state.session_repo.find(prior_session_id).await {
            Ok(prior_session) => {
                if let Some(ref csid) = prior_session.claude_session_id {
                    info!(task_id = %id, claude_session_id = %csid, "Found prior claude_session_id for resume");
                } else {
                    info!(task_id = %id, "No claude_session_id on prior session — will inject context");
                }
                prior_session.claude_session_id
            }
            Err(e) => {
                tracing::warn!(task_id = %id, error = %e, "Could not fetch prior session; falling back to context injection");
                None
            }
        }
    } else {
        None
    };

    let stage = task.stage.clone();
    info!(
        task_id = %id,
        title = %task.title,
        stage = %stage,
        comment_count = total_comments,
        reply_count = total_replies,
        has_context = conversation_context.is_some(),
        has_resume = resume_claude_session_id.is_some(),
        "Enqueuing continue session"
    );
    match queue.enqueue(task, stage, conversation_context, resume_claude_session_id).await {
        Ok(()) => {
            info!(task_id = %id, "Continue session enqueued");
            Json(serde_json::json!({ "status": "queued" })).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to enqueue continue session");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}

#[instrument(skip(state))]
async fn move_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
    Json(body): Json<MoveRequest>,
) -> impl IntoResponse {
    info!(task_id = %id, to_stage = %body.stage, "Moving task to new stage");
    match state.repo.move_to_stage(&id, &body.stage).await {
        Ok(task) => {
            info!(task_id = %id, title = %task.title, stage = %task.stage, "Task moved to stage");
            // Auto-stop session when task moves to Done
            if body.stage == "done" {
                if let Some(queue) = &state.queue {
                    if let Some(session_id) = queue.get_active_session_for_task(&id).await {
                        info!(task_id = %id, session_id = %session_id, "Auto-stopping session — task moved to Done");
                        let _ = queue.stop_session(&session_id).await;
                    }
                }
            }
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, to_stage = %body.stage, error = %e, "Failed to move task to stage");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
