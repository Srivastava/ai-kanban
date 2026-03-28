use crate::api::{AppState, AttachmentApiState, CommentApiState, LogApiState, PrometheusState, SessionApiState, SettingsApiState, TaskApiState};
use crate::api::analytics::{analytics_routes, AnalyticsApiState};
use crate::api::attachments::attachment_routes;
use crate::api::comments::{comment_routes, comment_standalone_routes};
use crate::api::logs::log_routes;
use crate::api::prometheus::metrics_handler;
use crate::api::sessions::session_routes;
use crate::api::settings::settings_routes;
use crate::api::tasks::task_routes;
use crate::static_files::static_handler;
use crate::ws::ws_handler;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    // Extract individual states for each API module
    let session_state: SessionApiState = state.clone().into();
    let task_state: TaskApiState = state.clone().into();
    let log_state: LogApiState = state.clone().into();
    let comment_state: CommentApiState = state.clone().into();
    let settings_state: SettingsApiState = state.clone().into();
    let analytics_state: AnalyticsApiState = state.clone().into();
    let prometheus_state = PrometheusState {
        otel_repo: state.otel_metrics.clone(),
        token_events: state.token_events.clone(),
    };

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let attachments_dir = std::env::var("ATTACHMENTS_DIR")
        .unwrap_or_else(|_| format!("{}/.ai-kanban/attachments", home));
    let attachment_state = AttachmentApiState {
        repo: state.attachments.clone(),
        task_repo: state.tasks.clone(),
        attachments_dir,
    };

    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .route("/ws", axum::routing::get(ws_handler))
        .route("/metrics", axum::routing::get(metrics_handler).with_state(prometheus_state))
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
        // Attachment routes nested under tasks
        .nest(
            "/api/tasks/:task_id/attachments",
            attachment_routes().with_state(attachment_state),
        )
        // Filesystem utility routes (stateless)
        .route("/api/fs/projects", axum::routing::get(crate::api::fs::list_projects))
        .fallback(static_handler)
}
