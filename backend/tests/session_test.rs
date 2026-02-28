use ai_kanban_backend::db::{create_pool, SessionRepository, TaskRepository};
use ai_kanban_backend::models::{CreateSession, CreateTask, SessionStatus, UpdateSession};

async fn setup_test_db() -> (TaskRepository, SessionRepository) {
    let db_path = format!("/tmp/test-session-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    (
        TaskRepository::new(pool.clone()),
        SessionRepository::new(pool),
    )
}

// ==================== Session Repository Tests ====================

#[tokio::test]
async fn test_session_create() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test Task".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession {
        task_id: task.id.clone(),
    }).await.unwrap();

    assert!(!session.id.is_empty());
    assert_eq!(session.task_id, task.id);
    assert_eq!(session.status, "pending");
}

#[tokio::test]
async fn test_session_find() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession {
        task_id: task.id.clone(),
    }).await.unwrap();

    let found = session_repo.find(&session.id).await.unwrap();
    assert_eq!(found.id, session.id);
}

#[tokio::test]
async fn test_session_find_not_found() {
    let (_, session_repo) = setup_test_db().await;

    let result = session_repo.find("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_list() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    // Create multiple sessions
    for _ in 0..3 {
        session_repo.create(CreateSession {
            task_id: task.id.clone(),
        }).await.unwrap();
    }

    let sessions = session_repo.list().await.unwrap();
    assert!(sessions.len() >= 3);
}

#[tokio::test]
async fn test_session_list_by_task() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task1 = task_repo.create(CreateTask {
        title: "Task 1".to_string(),
        description: None,
        project_path: "/tmp/test1".to_string(),
    }).await.unwrap();

    let task2 = task_repo.create(CreateTask {
        title: "Task 2".to_string(),
        description: None,
        project_path: "/tmp/test2".to_string(),
    }).await.unwrap();

    session_repo.create(CreateSession { task_id: task1.id.clone() }).await.unwrap();
    session_repo.create(CreateSession { task_id: task1.id.clone() }).await.unwrap();
    session_repo.create(CreateSession { task_id: task2.id.clone() }).await.unwrap();

    let task1_sessions = session_repo.list_by_task(&task1.id).await.unwrap();
    assert_eq!(task1_sessions.len(), 2);

    let task2_sessions = session_repo.list_by_task(&task2.id).await.unwrap();
    assert_eq!(task2_sessions.len(), 1);
}

#[tokio::test]
async fn test_session_update_status() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession {
        task_id: task.id.clone(),
    }).await.unwrap();

    let updated = session_repo.update(&session.id, UpdateSession {
        status: Some("running".to_string()),
        ..Default::default()
    }).await.unwrap();

    assert_eq!(updated.status, "running");
}

#[tokio::test]
async fn test_session_update_ended_at() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession {
        task_id: task.id.clone(),
    }).await.unwrap();

    let now = chrono::Utc::now();
    let updated = session_repo.update(&session.id, UpdateSession {
        ended_at: Some(now),
        ..Default::default()
    }).await.unwrap();

    assert!(updated.ended_at.is_some());
}

#[tokio::test]
async fn test_session_list_by_status() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let s1 = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();
    let s2 = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    // Update s1 to running
    session_repo.update(&s1.id, UpdateSession {
        status: Some("running".to_string()),
        ..Default::default()
    }).await.unwrap();

    let running = session_repo.list_by_status("running").await.unwrap();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, s1.id);

    let pending = session_repo.list_by_status("pending").await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, s2.id);
}

#[tokio::test]
async fn test_session_delete() {
    let (task_repo, session_repo) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession {
        task_id: task.id.clone(),
    }).await.unwrap();

    session_repo.delete(&session.id).await.unwrap();

    let result = session_repo.find(&session.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_delete_not_found() {
    let (_, session_repo) = setup_test_db().await;

    let result = session_repo.delete("nonexistent").await;
    assert!(result.is_err());
}

// ==================== SessionStatus Tests ====================

#[test]
fn test_session_status_as_str() {
    assert_eq!(SessionStatus::Pending.as_str(), "pending");
    assert_eq!(SessionStatus::Running.as_str(), "running");
    assert_eq!(SessionStatus::Stopped.as_str(), "stopped");
    assert_eq!(SessionStatus::Completed.as_str(), "completed");
    assert_eq!(SessionStatus::Failed.as_str(), "failed");
}

#[test]
fn test_session_status_from_str() {
    assert_eq!(SessionStatus::from_str("pending"), Some(SessionStatus::Pending));
    assert_eq!(SessionStatus::from_str("running"), Some(SessionStatus::Running));
    assert_eq!(SessionStatus::from_str("stopped"), Some(SessionStatus::Stopped));
    assert_eq!(SessionStatus::from_str("completed"), Some(SessionStatus::Completed));
    assert_eq!(SessionStatus::from_str("failed"), Some(SessionStatus::Failed));
    assert_eq!(SessionStatus::from_str("invalid"), None);
}

#[test]
fn test_session_status_all() {
    let all = SessionStatus::all();
    assert_eq!(all.len(), 5);
    assert!(all.contains(&"pending"));
    assert!(all.contains(&"running"));
    assert!(all.contains(&"stopped"));
    assert!(all.contains(&"completed"));
    assert!(all.contains(&"failed"));
}

#[test]
fn test_session_status_roundtrip() {
    for status_str in SessionStatus::all() {
        let status = SessionStatus::from_str(status_str).unwrap();
        assert_eq!(status.as_str(), *status_str);
    }
}
