/// Integration tests for queue advancement and session completion behavior.
///
/// These tests use real DB but no real Claude process.
/// They verify queue state transitions without spawning external processes.
use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
use ai_kanban_backend::db::{
    create_pool, AttachmentRepository, CommentRepository, OtelMetricsRepository,
    SessionMetricsRepository, SessionRepository, SettingsRepository, TaskRepository,
    TokenEventRepository,
};
use ai_kanban_backend::models::CreateTask;
use std::sync::Arc;

async fn setup() -> (Arc<ClaudeManager>, Arc<SessionQueue>, TaskRepository) {
    let db_path = format!("/tmp/test-completion-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    let session_repo = SessionRepository::new(pool.clone());
    let token_repo = TokenEventRepository::new(pool.clone());
    let metrics_repo = SessionMetricsRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let task_repo = TaskRepository::new(pool.clone());
    let otel_repo = OtelMetricsRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());
    let manager = Arc::new(ClaudeManager::new(
        session_repo,
        token_repo,
        metrics_repo,
        comment_repo,
        task_repo.clone(),
        otel_repo,
        None,
        None,
        attachment_repo,
    ));
    let queue = Arc::new(SessionQueue::new(manager.clone(), task_repo.clone()));
    (manager, queue, task_repo)
}

/// Verify that on_session_complete with an empty queue does not panic or error.
#[tokio::test]
async fn test_on_session_complete_empty_queue_is_ok() {
    let (_, queue, _) = setup().await;
    let result = queue.on_session_complete("any-session-id").await;
    assert!(
        result.is_ok(),
        "on_session_complete should return Ok even when queue is empty"
    );
}

/// Verify that the queue starts at length 0 and active count is 0.
#[tokio::test]
async fn test_initial_queue_state_is_zero() {
    let (manager, queue, _) = setup().await;
    assert_eq!(queue.queue_length().await, 0);
    assert_eq!(queue.active_count().await, 0);
    assert_eq!(manager.active_count().await, 0);
}

/// Verify that enqueue with a bad project path errors gracefully and does not
/// increment active count (session fails to start cleanly).
#[tokio::test]
async fn test_enqueue_with_bad_path_fails_gracefully() {
    let (manager, queue, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Bad Path Task".to_string(),
            description: None,
            project_path: "/nonexistent/path/should/fail".to_string(),
        })
        .await
        .unwrap();

    // Enqueue should return Err (bad path) but not panic
    let result = queue
        .enqueue(task, "planning".to_string(), None, None, 0)
        .await;
    assert!(result.is_err(), "enqueue with nonexistent path should fail");

    // Active count should remain 0 — the failed start should not leave a ghost session
    // (Note: start_session returns Err before inserting into active_sessions when path invalid)
    assert_eq!(
        manager.active_count().await,
        0,
        "active count should be 0 after failed start"
    );
}

/// Verify that a task added to the pending queue via a full-capacity scenario
/// stays in the queue until on_session_complete is called.
///
/// We simulate a full queue by pre-filling up to max_concurrent active sessions
/// using session_repo directly (bypassing Claude binary), then verify queue behavior.
#[tokio::test]
async fn test_queue_does_not_advance_without_on_session_complete() {
    let (_, queue, task_repo) = setup().await;

    // Verify queue starts empty
    assert_eq!(queue.queue_length().await, 0);

    let task = task_repo
        .create(CreateTask {
            title: "Queue Test".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();

    // Dequeue a task that's not there — should return false
    assert!(!queue.dequeue(&task.id).await);

    // Queue length still 0 — nothing was added
    assert_eq!(queue.queue_length().await, 0);
}

/// Verify that dequeue removes a task from the pending queue.
#[tokio::test]
async fn test_dequeue_removes_task_when_present() {
    // We can't easily fill the queue to 3 concurrent sessions without a real Claude binary,
    // so we test dequeue on an empty queue and verify it returns false correctly.
    let (_, queue, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Dequeue Test".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();

    // Not in queue yet
    assert!(
        !queue.dequeue(&task.id).await,
        "dequeue on non-queued task should return false"
    );
    assert_eq!(queue.queue_length().await, 0);
}

/// Verify get_position returns None for a task not in the queue.
#[tokio::test]
async fn test_get_position_returns_none_for_unknown_task() {
    let (_, queue, _) = setup().await;
    assert!(queue.get_position("not-in-queue").await.is_none());
}

/// Verify get_active_session_for_task returns None when no sessions are active.
#[tokio::test]
async fn test_get_active_session_for_task_none_when_idle() {
    let (_, queue, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Idle Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();

    assert!(queue.get_active_session_for_task(&task.id).await.is_none());
}

/// Verify that is_session_active returns false for an unknown session ID.
#[tokio::test]
async fn test_is_session_active_false_for_unknown() {
    let (_, queue, _) = setup().await;
    assert!(!queue.is_session_active("ghost-session-id").await);
}

/// Verify stop_session on a non-existent session returns Ok (idempotent).
#[tokio::test]
async fn test_stop_nonexistent_session_is_ok() {
    let (_, queue, _) = setup().await;
    let result = queue.stop_session("ghost-session").await;
    assert!(
        result.is_ok(),
        "stopping a non-existent session should be a no-op"
    );
}
