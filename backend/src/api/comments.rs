use super::CommentApiState;
use crate::models::{CreateComment, CreateCommentRequest};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use tracing::{debug, error, info, instrument};

#[derive(Deserialize, Debug)]
struct ListQuery {
    task_id: String,
}

pub fn comment_routes() -> Router<CommentApiState> {
    Router::new()
        .route("/", get(list_comments).post(create_comment))
        .route("/:id", delete(delete_comment))
}

#[instrument(skip(state))]
async fn list_comments(
    State(state): State<CommentApiState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    debug!(task_id = %query.task_id, "API: Listing comments for task");
    match state.repo.list_for_task(&query.task_id).await {
        Ok(comments) => {
            debug!(task_id = %query.task_id, count = comments.len(), "API: Comments retrieved");
            Json(comments).into_response()
        }
        Err(e) => {
            error!(task_id = %query.task_id, error = %e, "API: Failed to list comments");
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
    Json(req): Json<CreateCommentRequest>,
) -> impl IntoResponse {
    info!(task_id = %req.task_id, parent_id = ?req.parent_id, "API: Creating comment");
    // For now, author is "user" - later we'll detect Claude vs user
    let create = CreateComment {
        content: req.content,
        parent_id: req.parent_id,
    };
    match state.repo.create(&req.task_id, "user", create).await {
        Ok(comment) => {
            info!(comment_id = %comment.id, task_id = %comment.task_id, "API: Comment created");
            (StatusCode::CREATED, Json(comment)).into_response()
        }
        Err(e) => {
            error!(task_id = %req.task_id, error = %e, "API: Failed to create comment");
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
