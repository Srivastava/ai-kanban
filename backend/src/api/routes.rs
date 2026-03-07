use crate::api::{AppState, CommentApiState, LogApiState, SessionApiState, SettingsApiState, TaskApiState};
use crate::api::analytics::{analytics_routes, AnalyticsApiState};
use crate::api::comments::{comment_routes, comment_standalone_routes};
use crate::api::logs::log_routes;
use crate::api::sessions::session_routes;
use crate::api::settings::settings_routes;
use crate::api::tasks::task_routes;
use crate::ws::ws_handler;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    // Extract individual states for each API module
    let session_state: SessionApiState = state.clone().into();
    let task_state: TaskApiState = state.clone().into();
    let log_state: LogApiState = state.clone().into();
    let comment_state: CommentApiState = state.clone().into();
    let settings_state: SettingsApiState = state.clone().into();
    let analytics_state: AnalyticsApiState = state.into();

    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .route("/ws", axum::routing::get(ws_handler))
        // Session routes use SessionApiState
        .nest("/api/sessions", session_routes().with_state(session_state))
        // For tasks and logs, we convert to their respective states
        .nest("/api/tasks", task_routes().with_state(task_state))
        .nest("/api/logs", log_routes().with_state(log_state))
        // Analytics routes
        .nest("/api/analytics", analytics_routes().with_state(analytics_state))
        // Settings (feature flags)
        .nest("/api/settings", settings_routes().with_state(settings_state))
        // Comment routes nested under tasks (task_id comes from path)
        .nest(
            "/api/tasks/:task_id/comments",
            comment_routes().with_state(comment_state.clone()),
        )
        // Standalone comment routes (delete by ID)
        .nest("/api/comments", comment_standalone_routes().with_state(comment_state))
}
