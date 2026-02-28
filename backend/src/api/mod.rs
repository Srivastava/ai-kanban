use crate::db::{CommentRepository, LogRepository, SessionMetricsRepository, SessionRepository, TaskRepository, TokenEventRepository};
use crate::claude::SessionQueue;
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
    pub queue: Option<Arc<SessionQueue>>,
}

#[derive(Clone)]
pub struct TaskApiState {
    pub repo: TaskRepository,
    pub queue: Option<Arc<SessionQueue>>,
}

#[derive(Clone)]
pub struct LogApiState {
    pub repo: LogRepository,
}

#[derive(Clone)]
pub struct SessionApiState {
    pub queue: Arc<SessionQueue>,
}

#[derive(Clone)]
pub struct CommentApiState {
    pub repo: CommentRepository,
}

impl AppState {
    pub fn new(
        tasks: TaskRepository,
        logs: LogRepository,
        sessions: SessionRepository,
        comments: CommentRepository,
        token_events: TokenEventRepository,
        session_metrics: SessionMetricsRepository,
    ) -> Self {
        Self {
            tasks,
            logs,
            sessions,
            comments,
            token_events,
            session_metrics,
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

mod analytics;
mod comments;
mod logs;
mod routes;
mod sessions;
mod tasks;

pub use routes::create_router;
