use crate::api::AppState;
use crate::api::logs::log_routes;
use crate::api::tasks::task_routes;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .nest("/api/tasks", task_routes())
        .nest("/api/logs", log_routes())
        .with_state(state)
}
