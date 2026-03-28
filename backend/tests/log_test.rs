use ai_kanban_backend::db::{create_pool, LogRepository, TaskRepository};
use ai_kanban_backend::models::{level_to_str, CreateLog, CreateTask, Log, LogFilter};

async fn setup_test_db() -> (TaskRepository, LogRepository) {
    let db_path = format!("/tmp/test-logs-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    (TaskRepository::new(pool.clone()), LogRepository::new(pool))
}

#[tokio::test]
async fn test_create_log() {
    let (_, log_repo) = setup_test_db().await;

    let log = log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Test log message".to_string(),
            target: Some("test::module".to_string()),
            source: Some("backend".to_string()),
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .expect("Failed to create log");

    assert!(log.id > 0);
    assert_eq!(log.level, "INFO");
    assert_eq!(log.message, "Test log message");
    assert_eq!(log.source, "backend");
}

#[tokio::test]
async fn test_create_log_with_context() {
    let (_, log_repo) = setup_test_db().await;

    let log = log_repo
        .create(CreateLog {
            level: "ERROR".to_string(),
            message: "Task failed".to_string(),
            target: Some("api::tasks".to_string()),
            source: Some("backend".to_string()),
            task_id: Some("task-123".to_string()),
            session_id: Some("session-456".to_string()),
            metadata: Some(serde_json::json!({ "error_code": 500 })),
        })
        .await
        .expect("Failed to create log");

    assert_eq!(log.task_id, Some("task-123".to_string()));
    assert_eq!(log.session_id, Some("session-456".to_string()));
    assert!(log.metadata.is_some());
}

#[tokio::test]
async fn test_list_logs() {
    let (_, log_repo) = setup_test_db().await;

    // Create multiple logs
    for i in 0..5 {
        log_repo
            .create(CreateLog {
                level: "INFO".to_string(),
                message: format!("Log message {}", i),
                target: None,
                source: Some("backend".to_string()),
                task_id: None,
                session_id: None,
                metadata: None,
            })
            .await
            .unwrap();
    }

    let logs = log_repo
        .list(LogFilter::default())
        .await
        .expect("Failed to list logs");

    assert_eq!(logs.len(), 5);
}

#[tokio::test]
async fn test_filter_logs_by_level() {
    let (_, log_repo) = setup_test_db().await;

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Info log".to_string(),
            target: None,
            source: None,
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "ERROR".to_string(),
            message: "Error log".to_string(),
            target: None,
            source: None,
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    let error_logs = log_repo
        .list(LogFilter {
            level: Some("ERROR".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(error_logs.len(), 1);
    assert_eq!(error_logs[0].level, "ERROR");
}

#[tokio::test]
async fn test_filter_logs_by_source() {
    let (_, log_repo) = setup_test_db().await;

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Backend log".to_string(),
            target: None,
            source: Some("backend".to_string()),
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Frontend log".to_string(),
            target: None,
            source: Some("frontend".to_string()),
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    let frontend_logs = log_repo
        .list(LogFilter {
            source: Some("frontend".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(frontend_logs.len(), 1);
    assert_eq!(frontend_logs[0].source, "frontend");
}

#[tokio::test]
async fn test_filter_logs_by_task_id() {
    let (task_repo, log_repo) = setup_test_db().await;

    // Create a task
    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create logs for the task
    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Task started".to_string(),
            target: None,
            source: None,
            task_id: Some(task.id.clone()),
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Unrelated log".to_string(),
            target: None,
            source: None,
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    let task_logs = log_repo
        .list(LogFilter {
            task_id: Some(task.id.clone()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(task_logs.len(), 1);
    assert_eq!(task_logs[0].task_id, Some(task.id));
}

#[tokio::test]
async fn test_list_by_task() {
    let (task_repo, log_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    for i in 0..3 {
        log_repo
            .create(CreateLog {
                level: "INFO".to_string(),
                message: format!("Task log {}", i),
                target: None,
                source: None,
                task_id: Some(task.id.clone()),
                session_id: None,
                metadata: None,
            })
            .await
            .unwrap();
    }

    let logs = log_repo.list_by_task(&task.id, None).await.unwrap();
    assert_eq!(logs.len(), 3);
}

#[tokio::test]
async fn test_pagination() {
    let (_, log_repo) = setup_test_db().await;

    for i in 0..20 {
        log_repo
            .create(CreateLog {
                level: "INFO".to_string(),
                message: format!("Log {}", i),
                target: None,
                source: None,
                task_id: None,
                session_id: None,
                metadata: None,
            })
            .await
            .unwrap();
    }

    let page1 = log_repo
        .list(LogFilter {
            limit: Some(5),
            offset: Some(0),
            ..Default::default()
        })
        .await
        .unwrap();

    let page2 = log_repo
        .list(LogFilter {
            limit: Some(5),
            offset: Some(5),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page1.len(), 5);
    assert_eq!(page2.len(), 5);
    assert_ne!(page1[0].id, page2[0].id);
}

#[tokio::test]
async fn test_delete_old_logs() {
    let (_, log_repo) = setup_test_db().await;

    // Create some logs
    for i in 0..5 {
        log_repo
            .create(CreateLog {
                level: "INFO".to_string(),
                message: format!("Log {}", i),
                target: None,
                source: None,
                task_id: None,
                session_id: None,
                metadata: None,
            })
            .await
            .unwrap();
    }

    // Delete logs older than 1 day (should delete nothing since they're fresh)
    let deleted = log_repo.delete_old_logs(1).await.unwrap();
    assert_eq!(deleted, 0);

    // Verify logs still exist
    let logs = log_repo.list(LogFilter::default()).await.unwrap();
    assert_eq!(logs.len(), 5);
}

#[tokio::test]
async fn test_log_order_descending() {
    let (_, log_repo) = setup_test_db().await;

    // Create logs with slight delay to ensure different timestamps
    for i in 0..3 {
        log_repo
            .create(CreateLog {
                level: "INFO".to_string(),
                message: format!("Log {}", i),
                target: None,
                source: None,
                task_id: None,
                session_id: None,
                metadata: None,
            })
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let logs = log_repo.list(LogFilter::default()).await.unwrap();

    // Should be newest first
    assert!(logs[0].timestamp >= logs[1].timestamp);
    assert!(logs[1].timestamp >= logs[2].timestamp);
}

#[tokio::test]
async fn test_filter_logs_by_session_id() {
    let (_, log_repo) = setup_test_db().await;

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Session A log".to_string(),
            target: None,
            source: None,
            task_id: None,
            session_id: Some("session-a".to_string()),
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Session B log".to_string(),
            target: None,
            source: None,
            task_id: None,
            session_id: Some("session-b".to_string()),
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "No session log".to_string(),
            target: None,
            source: None,
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    let session_a_logs = log_repo
        .list(LogFilter {
            session_id: Some("session-a".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(session_a_logs.len(), 1);
    assert_eq!(session_a_logs[0].session_id, Some("session-a".to_string()));
}

#[tokio::test]
async fn test_list_by_task_with_limit() {
    let (task_repo, log_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    for i in 0..10 {
        log_repo
            .create(CreateLog {
                level: "INFO".to_string(),
                message: format!("Task log {}", i),
                target: None,
                source: None,
                task_id: Some(task.id.clone()),
                session_id: None,
                metadata: None,
            })
            .await
            .unwrap();
    }

    let logs = log_repo.list_by_task(&task.id, Some(5)).await.unwrap();
    assert_eq!(logs.len(), 5);
}

#[tokio::test]
async fn test_log_model_new() {
    let create = CreateLog {
        level: "DEBUG".to_string(),
        message: "Test message".to_string(),
        target: Some("test::module".to_string()),
        source: Some("test".to_string()),
        task_id: Some("task-1".to_string()),
        session_id: Some("session-1".to_string()),
        metadata: Some(serde_json::json!({ "key": "value" })),
    };

    let log = Log::new(create);

    assert_eq!(log.id, 0); // New logs have id 0 before database insert
    assert_eq!(log.level, "DEBUG");
    assert_eq!(log.message, "Test message");
    assert_eq!(log.target, Some("test::module".to_string()));
    assert_eq!(log.source, "test");
    assert_eq!(log.task_id, Some("task-1".to_string()));
    assert_eq!(log.session_id, Some("session-1".to_string()));
    assert!(log.metadata.is_some());
}

#[tokio::test]
async fn test_log_model_new_default_source() {
    let create = CreateLog {
        level: "INFO".to_string(),
        message: "Test".to_string(),
        target: None,
        source: None, // Should default to "backend"
        task_id: None,
        session_id: None,
        metadata: None,
    };

    let log = Log::new(create);
    assert_eq!(log.source, "backend");
}

#[test]
fn test_level_to_str_conversion() {
    assert_eq!(level_to_str("debug"), "DEBUG");
    assert_eq!(level_to_str("INFO"), "INFO");
    assert_eq!(level_to_str("warn"), "WARN");
    assert_eq!(level_to_str("warning"), "WARN");
    assert_eq!(level_to_str("ERROR"), "ERROR");
    assert_eq!(level_to_str("unknown"), "INFO"); // Unknown levels default to INFO
}

#[tokio::test]
async fn test_create_log_default_source() {
    let (_, log_repo) = setup_test_db().await;

    let log = log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "No source specified".to_string(),
            target: None,
            source: None, // Should default to "backend"
            task_id: None,
            session_id: None,
            metadata: None,
        })
        .await
        .expect("Failed to create log");

    assert_eq!(log.source, "backend");
}

#[tokio::test]
async fn test_combined_filters() {
    let (_, log_repo) = setup_test_db().await;

    // Create logs with different combinations
    log_repo
        .create(CreateLog {
            level: "ERROR".to_string(),
            message: "Backend error".to_string(),
            target: None,
            source: Some("backend".to_string()),
            task_id: Some("task-1".to_string()),
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "INFO".to_string(),
            message: "Backend info".to_string(),
            target: None,
            source: Some("backend".to_string()),
            task_id: Some("task-1".to_string()),
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    log_repo
        .create(CreateLog {
            level: "ERROR".to_string(),
            message: "Frontend error".to_string(),
            target: None,
            source: Some("frontend".to_string()),
            task_id: Some("task-1".to_string()),
            session_id: None,
            metadata: None,
        })
        .await
        .unwrap();

    // Filter by level AND source
    let logs = log_repo
        .list(LogFilter {
            level: Some("ERROR".to_string()),
            source: Some("backend".to_string()),
            task_id: Some("task-1".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].level, "ERROR");
    assert_eq!(logs[0].source, "backend");
}

#[tokio::test]
async fn test_empty_result() {
    let (_, log_repo) = setup_test_db().await;

    let logs = log_repo
        .list(LogFilter {
            level: Some("NONEXISTENT".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(logs.len(), 0);
}
