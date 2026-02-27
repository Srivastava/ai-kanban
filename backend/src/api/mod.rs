mod logs;
mod routes;
mod tasks;

pub use logs::LogApiState;
pub use routes::create_router;
pub use tasks::TaskApiState;

use crate::db::{LogRepository, TaskRepository};

#[derive(Clone)]
pub struct AppState {
    pub tasks: TaskRepository,
    pub logs: LogRepository,
}

impl AppState {
    pub fn new(tasks: TaskRepository, logs: LogRepository) -> Self {
        Self { tasks, logs }
    }
}

// Implement From<AppState> for individual states
impl From<AppState> for TaskApiState {
    fn from(state: AppState) -> Self {
        TaskApiState { repo: state.tasks }
    }
}

impl From<AppState> for LogApiState {
    fn from(state: AppState) -> Self {
        LogApiState { repo: state.logs }
    }
}
