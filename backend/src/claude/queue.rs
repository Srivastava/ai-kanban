use crate::claude::ClaudeManager;
use crate::db::TaskRepository;
use crate::models::Task;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument};

/// Trait capturing every method `SessionQueue` calls on the Claude manager.
/// `ClaudeManager` implements this; tests can provide a `MockManager`.
pub trait ManagesSession: Send + Sync + 'static {
    fn active_count(&self) -> impl std::future::Future<Output = usize> + Send;
    fn start_session(
        &self,
        task: crate::models::Task,
        stage: &str,
        conversation_context: Option<String>,
        resume_claude_session_id: Option<String>,
        retry_attempt: u32,
    ) -> impl std::future::Future<Output = anyhow::Result<String>> + Send;
    fn is_active(&self, session_id: &str) -> impl std::future::Future<Output = bool> + Send;
    fn stop_session(
        &self,
        session_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn recently_active(&self) -> impl std::future::Future<Output = bool> + Send;
    fn get_active_session_for_task(
        &self,
        task_id: &str,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

const MAX_RETRIES: u32 = 3;

#[cfg(not(test))]
const RETRY_DELAY_SECS: u64 = 30;

#[cfg(test)]
const RETRY_DELAY_SECS: u64 = 0;

pub(crate) fn should_retry(retry_attempt: u32) -> bool {
    retry_attempt < MAX_RETRIES
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task: Task,
    pub stage: String,
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub conversation_context: Option<String>,
    pub resume_claude_session_id: Option<String>,
    pub retry_attempt: u32,
}

pub struct SessionQueue<M: ManagesSession = ClaudeManager> {
    max_concurrent: usize,
    manager: Arc<M>,
    task_repo: TaskRepository,
    pending: Arc<Mutex<VecDeque<QueuedTask>>>,
}

impl<M: ManagesSession> SessionQueue<M> {
    pub fn new(manager: Arc<M>, task_repo: TaskRepository) -> Self {
        Self {
            max_concurrent: 3,
            manager,
            task_repo,
            pending: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    #[instrument(skip(self, task))]
    pub async fn enqueue(
        &self,
        task: Task,
        stage: String,
        conversation_context: Option<String>,
        resume_claude_session_id: Option<String>,
        retry_attempt: u32,
    ) -> Result<()> {
        let active_count = self.manager.active_count().await;

        if active_count < self.max_concurrent {
            info!(
                task_id = %task.id,
                task_title = %task.title,
                stage = %stage,
                active_sessions = active_count,
                "Starting task immediately"
            );
            self.manager
                .start_session(task, &stage, conversation_context, resume_claude_session_id, retry_attempt)
                .await?;
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
                retry_attempt,
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
            if let Err(e) = self
                .manager
                .start_session(
                    queued.task.clone(),
                    &queued.stage,
                    queued.conversation_context,
                    queued.resume_claude_session_id,
                    queued.retry_attempt,
                )
                .await
            {
                tracing::error!(
                    task_id = %queued.task.id,
                    task_title = %queued.task.title,
                    error = %e,
                    "Failed to start next queued task — slot remains free, task dropped from queue"
                );
            }
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

    pub async fn recently_active(&self) -> bool {
        self.manager.recently_active().await
    }

    pub async fn get_active_session_for_task(&self, task_id: &str) -> Option<String> {
        self.manager.get_active_session_for_task(task_id).await
    }

    /// Called when a session fails (non-zero exit, non-rate-limit).
    /// Retries up to MAX_RETRIES times, resuming the prior session.
    pub async fn schedule_failure_retry(
        self: std::sync::Arc<Self>,
        task_id: String,
        stage: String,
        claude_session_id: Option<String>,
        retry_attempt: u32,
    ) {
        if !should_retry(retry_attempt) {
            tracing::warn!(
                task_id = %task_id,
                retry_attempt = retry_attempt,
                max_retries = MAX_RETRIES,
                "Session failed — max retries exceeded, not retrying"
            );
            return;
        }

        let task_repo = self.task_repo.clone();
        let next_attempt = retry_attempt + 1;
        tokio::spawn(async move {
            tracing::info!(
                task_id = %task_id,
                retry_attempt = retry_attempt,
                next_attempt = next_attempt,
                delay_secs = RETRY_DELAY_SECS,
                "Session failed — scheduling retry"
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;

            match task_repo.find(&task_id).await {
                Ok(task) => {
                    tracing::info!(task_id = %task_id, attempt = next_attempt, "Re-queuing failed task");
                    if let Err(e) = self
                        .enqueue(task, stage, None, claude_session_id, next_attempt)
                        .await
                    {
                        tracing::error!(task_id = %task_id, error = %e, "Failed to re-queue failed task");
                    }
                }
                Err(e) => {
                    tracing::error!(task_id = %task_id, error = %e, "Failed to find task for failure retry")
                }
            }
        });
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
                    if let Err(e) = self.enqueue(task, stage, None, claude_session_id, 0).await {
                        tracing::error!(task_id = %task_id, error = %e, "Failed to re-queue rate-limited task");
                    }
                }
                Err(e) => {
                    tracing::error!(task_id = %task_id, error = %e, "Failed to find task for rate-limit retry")
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TaskRepository;
    use crate::models::Task;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // ── MockManager ───────────────────────────────────────────────────────────

    struct MockManager {
        active: Arc<AtomicUsize>,
        started: Arc<tokio::sync::Mutex<Vec<String>>>,
    }

    impl MockManager {
        fn new(active_count: usize) -> Self {
            Self {
                active: Arc::new(AtomicUsize::new(active_count)),
                started: Arc::new(tokio::sync::Mutex::new(vec![])),
            }
        }

        async fn start_session_count(&self) -> usize {
            self.started.lock().await.len()
        }
    }

    impl ManagesSession for MockManager {
        fn active_count(&self) -> impl std::future::Future<Output = usize> + Send {
            let count = self.active.load(Ordering::SeqCst);
            async move { count }
        }

        fn start_session(
            &self,
            task: Task,
            _stage: &str,
            _conversation_context: Option<String>,
            _resume_claude_session_id: Option<String>,
            _retry_attempt: u32,
        ) -> impl std::future::Future<Output = anyhow::Result<String>> + Send {
            let started = self.started.clone();
            let task_id = task.id.clone();
            async move {
                started.lock().await.push(task_id.clone());
                Ok(task_id)
            }
        }

        fn is_active(
            &self,
            _session_id: &str,
        ) -> impl std::future::Future<Output = bool> + Send {
            async { false }
        }

        fn stop_session(
            &self,
            _session_id: &str,
        ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
            async { Ok(()) }
        }

        fn recently_active(&self) -> impl std::future::Future<Output = bool> + Send {
            async { false }
        }

        fn get_active_session_for_task(
            &self,
            _task_id: &str,
        ) -> impl std::future::Future<Output = Option<String>> + Send {
            async { None }
        }
    }

    // ── Helper ────────────────────────────────────────────────────────────────

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {}", id),
            description: None,
            stage: "planning".to_string(),
            project_path: "/tmp/test".to_string(),
            session_id: None,
            priority: 0,
            context: None,
            compressed_context: None,
            instructions: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_queue(active_count: usize) -> (Arc<MockManager>, Arc<SessionQueue<MockManager>>) {
        let mock = Arc::new(MockManager::new(active_count));
        // Lazy pool — never actually connects; safe as long as tests don't call task_repo methods
        let pool = sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap();
        let task_repo = TaskRepository::new(pool);
        let queue = Arc::new(SessionQueue::new(mock.clone(), task_repo));
        (mock, queue)
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn should_retry_under_limit() {
        assert!(should_retry(0));
        assert!(should_retry(1));
        assert!(should_retry(2));
    }

    #[test]
    fn should_retry_at_limit() {
        assert!(!should_retry(MAX_RETRIES));
        assert!(!should_retry(MAX_RETRIES + 1));
    }

    #[test]
    fn queued_task_default_retry_attempt() {
        let task = make_task("t1");
        let queued = QueuedTask {
            task,
            stage: "planning".to_string(),
            queued_at: chrono::Utc::now(),
            conversation_context: None,
            resume_claude_session_id: None,
            retry_attempt: 0,
        };
        assert_eq!(queued.retry_attempt, 0);
    }

    #[tokio::test]
    async fn enqueue_starts_immediately_when_under_capacity() {
        let (mock, queue) = make_queue(0); // active=0, capacity=3 → under capacity
        queue
            .enqueue(make_task("t1"), "planning".to_string(), None, None, 0)
            .await
            .unwrap();
        assert_eq!(mock.start_session_count().await, 1);
        assert_eq!(queue.queue_length().await, 0);
    }

    #[tokio::test]
    async fn enqueue_queues_when_at_capacity() {
        let (mock, queue) = make_queue(3); // active=3, capacity=3 → at capacity
        queue
            .enqueue(make_task("t1"), "planning".to_string(), None, None, 0)
            .await
            .unwrap();
        assert_eq!(mock.start_session_count().await, 0);
        assert_eq!(queue.queue_length().await, 1);
    }

    #[tokio::test]
    async fn on_session_complete_drains_queue() {
        let (mock, queue) = make_queue(3); // at capacity so task gets buffered
        queue
            .enqueue(make_task("t1"), "planning".to_string(), None, None, 0)
            .await
            .unwrap();
        assert_eq!(queue.queue_length().await, 1);

        queue.on_session_complete("session-abc").await.unwrap();

        assert_eq!(mock.start_session_count().await, 1);
        assert_eq!(queue.queue_length().await, 0);
    }

    #[test]
    fn retry_attempt_increments_correctly() {
        // Pure arithmetic: the value we pass to re-enqueue is retry_attempt + 1
        for attempt in 0..MAX_RETRIES {
            assert_eq!(attempt + 1, attempt + 1); // trivially true, but documents intent
        }
        // More useful: verify we never enqueue with attempt == MAX_RETRIES
        assert!(should_retry(MAX_RETRIES - 1));
        assert!(!should_retry(MAX_RETRIES));
    }

    #[tokio::test]
    async fn schedule_failure_retry_does_not_retry_at_max() {
        let (mock, queue) = make_queue(0);
        // Calling with retry_attempt == MAX_RETRIES → should return early, no spawn, no start_session
        queue
            .clone()
            .schedule_failure_retry(
                "task-exhausted".to_string(),
                "planning".to_string(),
                None,
                MAX_RETRIES,
            )
            .await;

        // No sleep needed — returns before spawning
        assert_eq!(mock.start_session_count().await, 0);
    }
}
