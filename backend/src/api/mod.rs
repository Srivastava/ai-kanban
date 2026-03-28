use crate::claude::SessionQueue;
use crate::db::{
    AttachmentRepository, CommentRepository, LogRepository, OtelMetricsRepository,
    SessionMetricsRepository, SessionRepository, SettingsRepository, TaskRepository,
    TokenEventRepository,
};
use std::sync::Arc;

// Define state types first before the submodules
#[derive(Clone)]
pub struct AppState {
    pub tasks: TaskRepository,
    pub logs: LogRepository,
    pub sessions: SessionRepository,
    pub comments: CommentRepository,
    pub token_events: TokenEventRepository,
    pub session_metrics: SessionMetricsRepository,
    pub settings: SettingsRepository,
    pub otel_metrics: OtelMetricsRepository,
    pub attachments: AttachmentRepository,
    pub queue: Option<Arc<SessionQueue>>,
}

#[derive(Clone)]
pub struct TaskApiState {
    pub repo: TaskRepository,
    pub queue: Option<Arc<SessionQueue>>,
    pub comment_repo: CommentRepository,
    pub session_repo: SessionRepository,
}

#[derive(Clone)]
pub struct LogApiState {
    pub repo: LogRepository,
}

#[derive(Clone)]
pub struct SessionApiState {
    pub queue: Arc<SessionQueue>,
    pub session_repo: SessionRepository,
}

#[derive(Clone)]
pub struct CommentApiState {
    pub repo: CommentRepository,
}

#[derive(Clone)]
pub struct AttachmentApiState {
    pub repo: AttachmentRepository,
    pub task_repo: TaskRepository,
    pub attachments_dir: String,
}

impl AppState {
    pub fn new(
        tasks: TaskRepository,
        logs: LogRepository,
        sessions: SessionRepository,
        comments: CommentRepository,
        token_events: TokenEventRepository,
        session_metrics: SessionMetricsRepository,
        settings: SettingsRepository,
        otel_metrics: OtelMetricsRepository,
        attachments: AttachmentRepository,
    ) -> Self {
        Self {
            tasks,
            logs,
            sessions,
            comments,
            token_events,
            session_metrics,
            settings,
            otel_metrics,
            attachments,
            queue: None,
        }
    }

    pub fn with_queue(mut self, queue: Arc<SessionQueue>) -> Self {
        self.queue = Some(queue);
        self
    }
}

// Implement From<AppState> for individual states
impl From<AppState> for TaskApiState {
    fn from(state: AppState) -> Self {
        TaskApiState {
            repo: state.tasks,
            queue: state.queue,
            comment_repo: state.comments,
            session_repo: state.sessions,
        }
    }
}

impl From<AppState> for LogApiState {
    fn from(state: AppState) -> Self {
        LogApiState { repo: state.logs }
    }
}

impl From<AppState> for SessionApiState {
    fn from(state: AppState) -> Self {
        SessionApiState {
            queue: state.queue.expect("SessionQueue not initialized"),
            session_repo: state.sessions,
        }
    }
}

impl From<AppState> for CommentApiState {
    fn from(state: AppState) -> Self {
        CommentApiState {
            repo: state.comments,
        }
    }
}

impl From<AppState> for SettingsApiState {
    fn from(state: AppState) -> Self {
        SettingsApiState {
            repo: state.settings,
        }
    }
}

mod analytics;
pub mod attachments;
pub mod claude_jsonl;
pub mod claude_usage_cli;
mod comments;
pub mod fs;
mod logs;
pub mod otlp;
mod otlp_parser;
pub mod plan_tier;
pub mod prometheus;
mod routes;
mod sessions;
pub mod settings;
mod tasks;

pub use attachments::attachment_routes;
pub use otlp::{otlp_router, OtlpState};
pub use prometheus::PrometheusState;
pub use routes::create_router;
pub use settings::SettingsApiState;
