use crate::api::tasks::{task_routes, TaskApiState};
use axum::Router;

pub fn create_router(state: TaskApiState) -> Router {
    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .nest("/api/tasks", task_routes())
        .with_state(state)
}
