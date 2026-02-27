use crate::api::AppState;
use crate::models::{CreateTask, UpdateTask};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

#[derive(Clone)]
pub struct TaskApiState {
    pub repo: crate::db::TaskRepository,
}

#[derive(Deserialize)]
struct ListQuery {
    stage: Option<String>,
}

#[derive(Deserialize)]
struct MoveRequest {
    stage: String,
}

pub fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).patch(update_task).delete(delete_task))
        .route("/:id/move", post(move_task))
}

async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    match state.tasks.list(query.stage.as_deref()).await {
        Ok(tasks) => Json(tasks).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.tasks.find(&id).await {
        Ok(task) => Json(task).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn create_task(
    State(state): State<AppState>,
    Json(create): Json<CreateTask>,
) -> impl IntoResponse {
    match state.tasks.create(create).await {
        Ok(task) => (StatusCode::CREATED, Json(task)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<UpdateTask>,
) -> impl IntoResponse {
    match state.tasks.update(&id, update).await {
        Ok(task) => Json(task).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.tasks.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn move_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<MoveRequest>,
) -> impl IntoResponse {
    match state.tasks.move_to_stage(&id, &body.stage).await {
        Ok(task) => Json(task).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
