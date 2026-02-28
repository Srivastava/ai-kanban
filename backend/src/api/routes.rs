use crate::api::{AppState, CommentApiState, LogApiState, SessionApiState, TaskApiState};
use crate::api::comments::comment_routes;
use crate::api::logs::log_routes;
use crate::api::sessions::session_routes;
use crate::api::tasks::task_routes;
use crate::ws::ws_handler;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    // Extract individual states for each API module
    let session_state: SessionApiState = state.clone().into();
    let task_state: TaskApiState = state.clone().into();
    let log_state: LogApiState = state.clone().into();
    let comment_state: CommentApiState = state.into();

    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .route("/ws", axum::routing::get(ws_handler))
        // Session routes use SessionApiState
        .nest("/api/sessions", session_routes().with_state(session_state))
        // For tasks and logs, we convert to their respective states
        .nest("/api/tasks", task_routes().with_state(task_state))
        .nest("/api/logs", log_routes().with_state(log_state))
        // Comment routes - standalone endpoints for DELETE
        .nest("/api/comments", comment_routes().with_state(comment_state))
}
