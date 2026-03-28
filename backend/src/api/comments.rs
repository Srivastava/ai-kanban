use super::CommentApiState;
use crate::models::CreateComment;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use tracing::{debug, error, info, instrument};

/// Routes for comments nested under /api/tasks/:task_id/comments
pub fn comment_routes() -> Router<CommentApiState> {
    Router::new().route("/", get(list_comments).post(create_comment))
}

/// Routes for standalone comment operations (delete by ID)
pub fn comment_standalone_routes() -> Router<CommentApiState> {
    Router::new().route("/:id", delete(delete_comment))
}

#[instrument(skip(state))]
async fn list_comments(
    State(state): State<CommentApiState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    debug!(task_id = %task_id, "API: Listing comments for task");
    match state.repo.list_for_task(&task_id).await {
        Ok(comments) => {
            debug!(task_id = %task_id, count = comments.len(), "API: Comments retrieved");
            Json(comments).into_response()
        }
        Err(e) => {
            error!(task_id = %task_id, error = %e, "API: Failed to list comments");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn create_comment(
    State(state): State<CommentApiState>,
    Path(task_id): Path<String>,
    Json(data): Json<CreateComment>,
) -> impl IntoResponse {
    info!(task_id = %task_id, parent_id = ?data.parent_id, "API: Creating comment");
    // For now, author is "user" - later we'll detect Claude vs user
    match state.repo.create(&task_id, "user", data).await {
        Ok(comment) => {
            info!(comment_id = %comment.id, task_id = %task_id, "API: Comment created");
            (StatusCode::CREATED, Json(comment)).into_response()
        }
        Err(e) => {
            error!(task_id = %task_id, error = %e, "API: Failed to create comment");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn delete_comment(
    State(state): State<CommentApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(comment_id = %id, "API: Deleting comment");
    match state.repo.delete(&id).await {
        Ok(()) => {
            info!(comment_id = %id, "API: Comment deleted");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            error!(comment_id = %id, error = %e, "API: Failed to delete comment");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
