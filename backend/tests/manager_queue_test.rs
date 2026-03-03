use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
use ai_kanban_backend::db::{
    create_pool, CommentRepository, SessionMetricsRepository, SessionRepository,
    TaskRepository, TokenEventRepository,
};
use ai_kanban_backend::models::CreateTask;
use std::sync::Arc;

async fn setup() -> (Arc<ClaudeManager>, Arc<SessionQueue>, TaskRepository) {
    let db_path = format!("/tmp/test-mgr-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    let session_repo = SessionRepository::new(pool.clone());
    let token_repo = TokenEventRepository::new(pool.clone());
    let metrics_repo = SessionMetricsRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let task_repo = TaskRepository::new(pool.clone());
    let manager = Arc::new(ClaudeManager::new(
        session_repo,
        token_repo,
        metrics_repo,
        comment_repo,
        task_repo.clone(),
    ));
    let queue = Arc::new(SessionQueue::new(manager.clone(), task_repo.clone()));
    (manager, queue, task_repo)
}

#[tokio::test]
async fn test_manager_new_has_zero_active() {
    let (manager, _, _) = setup().await;
    assert_eq!(manager.active_count().await, 0);
}

#[tokio::test]
async fn test_manager_is_active_returns_false_for_unknown() {
    let (manager, _, _) = setup().await;
    assert!(!manager.is_active("nonexistent-session").await);
}

#[tokio::test]
async fn test_manager_get_active_session_for_task_returns_none() {
    let (manager, _, _) = setup().await;
    assert!(manager.get_active_session_for_task("some-task-id").await.is_none());
}

#[tokio::test]
async fn test_manager_stop_nonexistent_session_ok() {
    let (manager, _, _) = setup().await;
    let result = manager.stop_session("nonexistent-id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_manager_subscribe_creates_receiver() {
    let (manager, _, _) = setup().await;
    let _rx = manager.subscribe();
}

#[tokio::test]
async fn test_queue_starts_empty() {
    let (_, queue, _) = setup().await;
    assert_eq!(queue.queue_length().await, 0);
    assert!(queue.get_queued_tasks().await.is_empty());
}

#[tokio::test]
async fn test_queue_active_count_zero_initially() {
    let (_, queue, _) = setup().await;
    assert_eq!(queue.active_count().await, 0);
}

#[tokio::test]
async fn test_queue_get_position_returns_none_when_empty() {
    let (_, queue, _) = setup().await;
    assert!(queue.get_position("some-task-id").await.is_none());
}

#[tokio::test]
async fn test_queue_dequeue_returns_false_when_empty() {
    let (_, queue, _) = setup().await;
    assert!(!queue.dequeue("nonexistent-id").await);
}

#[tokio::test]
async fn test_queue_is_session_active_false_for_unknown() {
    let (_, queue, _) = setup().await;
    assert!(!queue.is_session_active("unknown-session").await);
}

#[tokio::test]
async fn test_queue_get_active_session_for_task_none() {
    let (_, queue, _) = setup().await;
    assert!(queue.get_active_session_for_task("task-id").await.is_none());
}

#[tokio::test]
async fn test_queue_stop_nonexistent_session_ok() {
    let (_, queue, _) = setup().await;
    let result = queue.stop_session("nonexistent").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_queue_enqueue_fails_gracefully_without_claude_binary() {
    let (_, queue, task_repo) = setup().await;
    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/nonexistent/path/that/does/not/exist".to_string(),
    }).await.unwrap();
    // Will error (no Claude binary / bad path) — must not panic
    let _result = queue.enqueue(task, "planning".to_string(), None, None).await;
}

#[tokio::test]
async fn test_queue_on_session_complete_empty_queue_is_ok() {
    let (_, queue, _) = setup().await;
    let result = queue.on_session_complete("any-session-id").await;
    assert!(result.is_ok());
}
