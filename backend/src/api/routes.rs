use crate::api::analytics::{analytics_routes, AnalyticsApiState};
use crate::api::attachments::attachment_routes;
use crate::api::comments::{comment_routes, comment_standalone_routes};
use crate::api::logs::log_routes;
use crate::api::prometheus::metrics_handler;
use crate::api::sessions::session_routes;
use crate::api::settings::settings_routes;
use crate::api::tasks::task_routes;
use crate::api::{
    AppState, AttachmentApiState, CommentApiState, HealthApiState, LogApiState, PrometheusState,
    SessionApiState, SettingsApiState, TaskApiState,
};
use crate::static_files::static_handler;
use crate::ws::ws_handler;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::Router;

async fn health_handler(State(health): State<HealthApiState>) -> impl IntoResponse {
    use crate::metrics::{DB_POOL_TIMEOUTS, ZOMBIE_SESSIONS_RECOVERED};
    use std::sync::atomic::Ordering;

    let mut status = "ok";
    let mut db_status = "unchecked";
    let mut db_error: Option<String> = None;

    if let Some(pool) = &health.pool {
        match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => db_status = "ok",
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("pool timed out") || msg.contains("PoolTimedOut") {
                    crate::metrics::record_pool_timeout();
                }
                tracing::error!(error = %e, "Health check: DB query failed");
                db_status = "error";
                db_error = Some(msg);
                status = "degraded";
            }
        }
    }

    let active_sessions = if let Some(q) = &health.queue {
        q.active_count().await as u64
    } else {
        0
    };
    let queued_tasks = if let Some(q) = &health.queue {
        q.queue_length().await as u64
    } else {
        0
    };

    let mut body = serde_json::json!({
        "status": status,
        "db": db_status,
        "active_sessions": active_sessions,
        "queued_tasks": queued_tasks,
        "pool_timeouts_since_startup": DB_POOL_TIMEOUTS.load(Ordering::Relaxed),
        "zombie_sessions_recovered": ZOMBIE_SESSIONS_RECOVERED.load(Ordering::Relaxed),
        "version": env!("CARGO_PKG_VERSION"),
    });

    if let Some(err) = db_error {
        body["db_error"] = serde_json::Value::String(err);
        return (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response();
    }

    Json(body).into_response()
}

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

    let health_state: HealthApiState = state.clone().into();

    Router::new()
        .route(
            "/health",
            axum::routing::get(health_handler).with_state(health_state),
        )
        .route("/ws", axum::routing::get(ws_handler))
        .route(
            "/metrics",
            axum::routing::get(metrics_handler).with_state(prometheus_state),
        )
        // Session routes use SessionApiState
        .nest("/api/sessions", session_routes().with_state(session_state))
        // For tasks and logs, we convert to their respective states
        .nest("/api/tasks", task_routes().with_state(task_state))
        .nest("/api/logs", log_routes().with_state(log_state))
        // Analytics routes
        .nest(
            "/api/analytics",
            analytics_routes().with_state(analytics_state),
        )
        // Settings (feature flags)
        .nest(
            "/api/settings",
            settings_routes().with_state(settings_state),
        )
        // Comment routes nested under tasks (task_id comes from path)
        .nest(
            "/api/tasks/:task_id/comments",
            comment_routes().with_state(comment_state.clone()),
        )
        // Standalone comment routes (delete by ID)
        .nest(
            "/api/comments",
            comment_standalone_routes().with_state(comment_state),
        )
        // Attachment routes nested under tasks
        .nest(
            "/api/tasks/:task_id/attachments",
            attachment_routes().with_state(attachment_state),
        )
        // Filesystem utility routes (stateless)
        .route(
            "/api/fs/projects",
            axum::routing::get(crate::api::fs::list_projects),
        )
        .fallback(static_handler)
}
