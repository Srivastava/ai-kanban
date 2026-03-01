use crate::claude::ClaudeManager;
use crate::db::TaskRepository;
use crate::models::Task;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument};

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task: Task,
    pub stage: String,
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub conversation_context: Option<String>,
}

pub struct SessionQueue {
    max_concurrent: usize,
    manager: Arc<ClaudeManager>,
    task_repo: TaskRepository,
    pending: Arc<Mutex<VecDeque<QueuedTask>>>,
}

impl SessionQueue {
    pub fn new(manager: Arc<ClaudeManager>, task_repo: TaskRepository) -> Self {
        Self {
            max_concurrent: 3,
            manager,
            task_repo,
            pending: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    #[instrument(skip(self, task))]
    pub async fn enqueue(&self, task: Task, stage: String, conversation_context: Option<String>) -> Result<()> {
        let active_count = self.manager.active_count().await;

        if active_count < self.max_concurrent {
            info!(task_id = %task.id, "Starting task immediately");
            self.manager.start_session(task, &stage, conversation_context).await?;
        } else {
            info!(task_id = %task.id, "Queuing task ({} active)", active_count);
            let mut pending = self.pending.lock().await;
            pending.push_back(QueuedTask {
                task,
                stage,
                queued_at: chrono::Utc::now(),
                conversation_context,
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn on_session_complete(&self, session_id: &str) -> Result<()> {
        info!(session_id = %session_id, "Session completed, checking queue");

        let mut pending = self.pending.lock().await;

        if let Some(queued) = pending.pop_front() {
            info!(task_id = %queued.task.id, "Starting queued task");
            drop(pending);
            self.manager.start_session(queued.task, &queued.stage, queued.conversation_context).await?;
        }

        Ok(())
    }

    pub async fn queue_length(&self) -> usize {
        self.pending.lock().await.len()
    }

    pub async fn get_queued_tasks(&self) -> Vec<QueuedTask> {
        self.pending.lock().await.iter().cloned().collect()
    }

    pub async fn dequeue(&self, task_id: &str) -> bool {
        let mut pending = self.pending.lock().await;
        let initial_len = pending.len();
        pending.retain(|qt| qt.task.id != task_id);
        pending.len() != initial_len
    }

    pub async fn get_position(&self, task_id: &str) -> Option<usize> {
        let pending = self.pending.lock().await;
        pending.iter().position(|qt| qt.task.id == task_id)
    }

    pub async fn is_session_active(&self, session_id: &str) -> bool {
        self.manager.is_active(session_id).await
    }

    pub async fn stop_session(&self, session_id: &str) -> anyhow::Result<()> {
        self.manager.stop_session(session_id).await
    }

    pub async fn active_count(&self) -> usize {
        self.manager.active_count().await
    }

    pub async fn get_active_session_for_task(&self, task_id: &str) -> Option<String> {
        self.manager.get_active_session_for_task(task_id).await
    }
}
