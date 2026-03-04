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
    pub resume_claude_session_id: Option<String>,
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
    pub async fn enqueue(&self, task: Task, stage: String, conversation_context: Option<String>, resume_claude_session_id: Option<String>) -> Result<()> {
        let active_count = self.manager.active_count().await;

        if active_count < self.max_concurrent {
            info!(
                task_id = %task.id,
                task_title = %task.title,
                stage = %stage,
                active_sessions = active_count,
                "Starting task immediately"
            );
            self.manager.start_session(task, &stage, conversation_context, resume_claude_session_id).await?;
        } else {
            let queue_len = self.pending.lock().await.len();
            info!(
                task_id = %task.id,
                task_title = %task.title,
                stage = %stage,
                active_sessions = active_count,
                queue_position = queue_len + 1,
                "Task queued — at capacity"
            );
            let mut pending = self.pending.lock().await;
            pending.push_back(QueuedTask {
                task,
                stage,
                queued_at: chrono::Utc::now(),
                conversation_context,
                resume_claude_session_id,
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn on_session_complete(&self, session_id: &str) -> Result<()> {
        info!(session_id = %session_id, "Session completed, checking queue");

        let mut pending = self.pending.lock().await;

        if let Some(queued) = pending.pop_front() {
            let remaining = pending.len();
            info!(
                task_id = %queued.task.id,
                task_title = %queued.task.title,
                stage = %queued.stage,
                remaining_in_queue = remaining,
                "Starting next queued task"
            );
            drop(pending);
            self.manager.start_session(queued.task, &queued.stage, queued.conversation_context, queued.resume_claude_session_id).await?;
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

    /// Called when a session hits a Claude usage limit.
    /// Sleeps until `reset_at` (+ 5s buffer) then re-enqueues the task using --resume.
    pub async fn schedule_rate_limit_retry(
        self: std::sync::Arc<Self>,
        task_id: String,
        stage: String,
        claude_session_id: Option<String>,
        reset_at: chrono::DateTime<chrono::Utc>,
    ) {
        let task_repo = self.task_repo.clone();
        tokio::spawn(async move {
            let now = chrono::Utc::now();
            let wait_secs = ((reset_at - now).num_seconds()).max(0) as u64 + 5;
            tracing::info!(
                task_id = %task_id,
                reset_at = %reset_at,
                wait_secs = wait_secs,
                "Rate limit detected — scheduling retry"
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;

            match task_repo.find(&task_id).await {
                Ok(task) => {
                    tracing::info!(task_id = %task_id, "Rate limit reset — re-queuing task");
                    if let Err(e) = self.enqueue(task, stage, None, claude_session_id).await {
                        tracing::error!(task_id = %task_id, error = %e, "Failed to re-queue rate-limited task");
                    }
                }
                Err(e) => tracing::error!(task_id = %task_id, error = %e, "Failed to find task for rate-limit retry"),
            }
        });
    }
}
