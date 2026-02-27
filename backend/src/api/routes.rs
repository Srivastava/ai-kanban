use crate::api::{AppState, LogApiState, SessionApiState, TaskApiState};
use crate::api::logs::log_routes;
use crate::api::sessions::session_routes;
use crate::api::tasks::task_routes;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    // Extract individual states for each API module
    let session_state: SessionApiState = state.clone().into();
    let task_state: TaskApiState = state.clone().into();
    let log_state: LogApiState = state.into();

    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        // Session routes use SessionApiState
        .nest("/api/sessions", session_routes().with_state(session_state))
        // For tasks and logs, we convert to their respective states
        .nest("/api/tasks", task_routes().with_state(task_state))
        .nest("/api/logs", log_routes().with_state(log_state))
}
