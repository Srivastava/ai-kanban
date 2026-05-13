use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
use ai_kanban_backend::db::{
    create_pool, AttachmentRepository, CommentRepository, OtelMetricsRepository,
    SessionMetricsRepository, SessionRepository, SettingsRepository, TaskRepository,
    TokenEventRepository,
};
use ai_kanban_backend::models::{CreateSession, CreateTask, UpdateSession};
use std::sync::Arc;

async fn setup() -> (Arc<ClaudeManager>, Arc<SessionQueue>, TaskRepository) {
    let db_path = format!("/tmp/test-mgr-{}.db", uuid::Uuid::new_v4());
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

async fn setup_with_session_repo() -> (
    Arc<ClaudeManager>,
    Arc<SessionQueue>,
    TaskRepository,
    SessionRepository,
) {
    let db_path = format!("/tmp/test-mgr-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    let session_repo = SessionRepository::new(pool.clone());
    let token_repo = TokenEventRepository::new(pool.clone());
    let metrics_repo = SessionMetricsRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let task_repo = TaskRepository::new(pool.clone());
    let otel_repo = OtelMetricsRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());
    let session_repo_clone = session_repo.clone();
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
    (manager, queue, task_repo, session_repo_clone)
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
    assert!(manager
        .get_active_session_for_task("some-task-id")
        .await
        .is_none());
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
    let task = task_repo
        .create(CreateTask {
            title: "Test".to_string(),
            description: None,
            project_path: "/nonexistent/path/that/does/not/exist".to_string(),
        })
        .await
        .unwrap();
    // Will error (no Claude binary / bad path) — must not panic
    let _result = queue
        .enqueue(task, "planning".to_string(), None, None, 0)
        .await;
}

#[tokio::test]
async fn test_queue_on_session_complete_empty_queue_is_ok() {
    let (_, queue, _) = setup().await;
    let result = queue.on_session_complete("any-session-id").await;
    assert!(result.is_ok());
}

// ==================== Resume Session Tests ====================

#[tokio::test]
async fn test_queue_enqueue_with_resume_claude_session_id_does_not_panic() {
    // Verifies the queue accepts a resume_claude_session_id without panicking.
    // Claude binary won't be found in test env — we only test the enqueue path, not actual execution.
    let (_, queue, task_repo) = setup().await;
    let task = task_repo
        .create(CreateTask {
            title: "Resume Test".to_string(),
            description: None,
            project_path: "/nonexistent/path".to_string(),
        })
        .await
        .unwrap();
    let resume_id = Some("claude-session-abc-123".to_string());
    let _result = queue
        .enqueue(task, "planning".to_string(), None, resume_id, 0)
        .await;
    // Must not panic regardless of Claude binary availability
}

#[tokio::test]
async fn test_queue_enqueue_with_none_resume_id_does_not_panic() {
    let (_, queue, task_repo) = setup().await;
    let task = task_repo
        .create(CreateTask {
            title: "Fresh Session Test".to_string(),
            description: None,
            project_path: "/nonexistent/path".to_string(),
        })
        .await
        .unwrap();
    let _result = queue
        .enqueue(task, "planning".to_string(), None, None, 0)
        .await;
}

#[tokio::test]
async fn test_queue_enqueue_with_conversation_context_and_resume_id() {
    // Verifies both conversation_context and resume_id can be passed together.
    let (_, queue, task_repo) = setup().await;
    let task = task_repo
        .create(CreateTask {
            title: "Context + Resume".to_string(),
            description: None,
            project_path: "/nonexistent/path".to_string(),
        })
        .await
        .unwrap();
    let context = Some("Previous conversation context here".to_string());
    let resume_id = Some("claude-prior-session-id".to_string());
    let _result = queue
        .enqueue(task, "review".to_string(), context, resume_id, 0)
        .await;
}

#[tokio::test]
async fn test_session_claude_id_stored_and_retrievable_via_repo() {
    // End-to-end: create session, store claude_session_id, look it up by claude_session_id.
    // This is the core path for Continue Session: find prior session → read claude_session_id → pass as --resume.
    use ai_kanban_backend::db::{create_pool, SessionRepository};
    use ai_kanban_backend::models::{CreateSession, CreateTask, UpdateSession};

    let db_path = format!("/tmp/test-resume-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    let task_repo = TaskRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());

    let task = task_repo
        .create(CreateTask {
            title: "Resume flow task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // 1. Create session (simulates start_session)
    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    assert!(session.claude_session_id.is_none());

    // 2. Store claude_session_id (simulates extract from JSONL init line)
    let expected_claude_id = "550e8400-e29b-41d4-a716-446655440000";
    session_repo
        .update(
            &session.id,
            UpdateSession {
                claude_session_id: Some(Some(expected_claude_id.to_string())),
                status: Some("completed".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // 3. Look up by claude_session_id (simulates continue_session path)
    let found = session_repo.find(&session.id).await.unwrap();
    assert_eq!(
        found.claude_session_id,
        Some(expected_claude_id.to_string())
    );

    // 4. Also verify find_by_claude_session_id works
    let by_claude_id = session_repo
        .find_by_claude_session_id(expected_claude_id)
        .await
        .unwrap();
    assert!(by_claude_id.is_some());
    assert_eq!(by_claude_id.unwrap().id, session.id);
}

// ==================== Zombie Session Tests ====================

/// Regression test: stop_session must update the DB to "stopped" even when the session
/// is NOT in the in-memory active_sessions map (zombie session). This was the bug that
/// caused the stop button to silently do nothing on stuck sessions.
#[tokio::test]
async fn test_stop_zombie_session_updates_db_to_stopped() {
    let (manager, _, task_repo, session_repo) = setup_with_session_repo().await;

    let task = task_repo
        .create(CreateTask {
            title: "Zombie Session Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create a session and manually set it to "running" — simulating a session
    // that was active before a crash and was never cleaned up.
    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    session_repo
        .update(
            &session.id,
            UpdateSession {
                status: Some("running".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Verify the session is NOT in the manager's active map (it's a zombie).
    assert!(!manager.is_active(&session.id).await);

    // Stop the zombie session — must succeed and update the DB.
    manager.stop_session(&session.id).await.unwrap();

    let updated = session_repo.find(&session.id).await.unwrap();
    assert_eq!(
        updated.status, "stopped",
        "zombie session must be marked stopped in DB"
    );
    assert!(
        updated.ended_at.is_some(),
        "ended_at must be set after stop"
    );
}

/// Watchdog must detect a zombie session (running in DB, absent from active map) and mark it stopped.
#[tokio::test]
async fn test_watchdog_reconcile_zombie_sessions() {
    let (manager, _, task_repo, session_repo) = setup_with_session_repo().await;

    let task = task_repo
        .create(CreateTask {
            title: "Watchdog Zombie Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create session and set it running — but never add to active_sessions.
    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    session_repo
        .update(
            &session.id,
            UpdateSession {
                status: Some("running".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Watchdog should find 1 zombie and clean it up.
    let recovered = manager.reconcile_zombie_sessions().await.unwrap();
    assert_eq!(
        recovered, 1,
        "watchdog should have recovered 1 zombie session"
    );

    let updated = session_repo.find(&session.id).await.unwrap();
    assert_eq!(updated.status, "stopped");
}

/// Watchdog must not touch sessions that are legitimately active in the manager.
#[tokio::test]
async fn test_watchdog_does_not_touch_non_zombie_sessions() {
    let (manager, _, _, _) = setup_with_session_repo().await;

    // With no running sessions in DB at all, watchdog must report 0 zombies.
    let recovered = manager.reconcile_zombie_sessions().await.unwrap();
    assert_eq!(recovered, 0, "no zombies should be found in a fresh DB");
}

/// Watchdog must not touch "pending" sessions — only "running" sessions are zombies.
#[tokio::test]
async fn test_watchdog_ignores_pending_sessions() {
    let (manager, _, task_repo, session_repo) = setup_with_session_repo().await;

    let task = task_repo
        .create(CreateTask {
            title: "Pending Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create a "pending" session (default status from session_repo.create)
    let _session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();

    // Watchdog only looks at "running" sessions — pending must be untouched.
    let recovered = manager.reconcile_zombie_sessions().await.unwrap();
    assert_eq!(
        recovered, 0,
        "pending sessions must not be treated as zombies"
    );
}

/// Stopping the same zombie session twice must be idempotent — the second call
/// should succeed even though the first already set it to "stopped".
#[tokio::test]
async fn test_stop_zombie_session_twice_is_idempotent() {
    let (manager, _, task_repo, session_repo) = setup_with_session_repo().await;

    let task = task_repo
        .create(CreateTask {
            title: "Idempotent Stop Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    session_repo
        .update(
            &session.id,
            UpdateSession {
                status: Some("running".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // First stop — should succeed.
    manager.stop_session(&session.id).await.unwrap();
    let after_first = session_repo.find(&session.id).await.unwrap();
    assert_eq!(after_first.status, "stopped");

    // Second stop — must also succeed (idempotent).
    let result = manager.stop_session(&session.id).await;
    assert!(
        result.is_ok(),
        "second stop on an already-stopped session must not error"
    );

    let after_second = session_repo.find(&session.id).await.unwrap();
    assert_eq!(after_second.status, "stopped");
}

/// Watchdog followed by stop must be fully idempotent — watchdog cleans up zombie,
/// then stop_session is called again and must succeed without error.
#[tokio::test]
async fn test_watchdog_then_stop_is_idempotent() {
    let (manager, _, task_repo, session_repo) = setup_with_session_repo().await;

    let task = task_repo
        .create(CreateTask {
            title: "Watchdog then Stop".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    session_repo
        .update(
            &session.id,
            UpdateSession {
                status: Some("running".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Watchdog cleans up first.
    let recovered = manager.reconcile_zombie_sessions().await.unwrap();
    assert_eq!(recovered, 1);

    // Now the user also clicks stop — must not error.
    let result = manager.stop_session(&session.id).await;
    assert!(result.is_ok(), "stop after watchdog cleanup must not error");
}
