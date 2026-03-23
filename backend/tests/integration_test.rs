use ai_kanban_backend::api::{AppState, LogApiState, SessionApiState, TaskApiState};
use ai_kanban_backend::db::{create_pool, AttachmentRepository, CommentRepository, LogRepository, OtelMetricsRepository, SessionMetricsRepository, SessionRepository, SettingsRepository, TaskRepository, TokenEventRepository};
use ai_kanban_backend::models::{CreateLog, CreateTask, Log, LogFilter, Stage, Task, UpdateTask};

async fn setup_test_db() -> (TaskRepository, LogRepository, SessionRepository, CommentRepository, TokenEventRepository, SessionMetricsRepository, SettingsRepository, OtelMetricsRepository, AttachmentRepository) {
    let db_path = format!("/tmp/test-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    (
        TaskRepository::new(pool.clone()),
        LogRepository::new(pool.clone()),
        SessionRepository::new(pool.clone()),
        CommentRepository::new(pool.clone()),
        TokenEventRepository::new(pool.clone()),
        SessionMetricsRepository::new(pool.clone()),
        SettingsRepository::new(pool.clone()),
        OtelMetricsRepository::new(pool.clone()),
        AttachmentRepository::new(pool),
    )
}

#[tokio::test]
async fn test_create_task() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let create = CreateTask {
        title: "Test Task".to_string(),
        description: Some("Test description".to_string()),
        project_path: "/tmp/test-project".to_string(),
    };

    let task = repo.create(create).await.expect("Failed to create task");

    assert!(!task.id.is_empty());
    assert_eq!(task.title, "Test Task");
    assert_eq!(task.stage, "backlog");
}

#[tokio::test]
async fn test_list_tasks() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    // Create multiple tasks
    repo.create(CreateTask {
        title: "Task 1".to_string(),
        description: None,
        project_path: "/tmp/project".to_string(),
    })
    .await
    .unwrap();

    repo.create(CreateTask {
        title: "Task 2".to_string(),
        description: None,
        project_path: "/tmp/project".to_string(),
    })
    .await
    .unwrap();

    let tasks = repo.list(None).await.expect("Failed to list tasks");
    assert_eq!(tasks.len(), 2);
}

#[tokio::test]
async fn test_filter_by_stage() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task1 = repo
        .create(CreateTask {
            title: "Backlog Task".to_string(),
            description: None,
            project_path: "/tmp/project".to_string(),
        })
        .await
        .unwrap();

    let task2 = repo
        .create(CreateTask {
            title: "Ready Task".to_string(),
            description: None,
            project_path: "/tmp/project".to_string(),
        })
        .await
        .unwrap();

    repo.move_to_stage(&task2.id, "ready").await.unwrap();

    let backlog_tasks = repo.list(Some("backlog")).await.unwrap();
    assert_eq!(backlog_tasks.len(), 1);
    assert_eq!(backlog_tasks[0].id, task1.id);

    let ready_tasks = repo.list(Some("ready")).await.unwrap();
    assert_eq!(ready_tasks.len(), 1);
    assert_eq!(ready_tasks[0].id, task2.id);
}

#[tokio::test]
async fn test_update_task() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo
        .create(CreateTask {
            title: "Original Title".to_string(),
            description: None,
            project_path: "/tmp/project".to_string(),
        })
        .await
        .unwrap();

    let updated = repo
        .update(
            &task.id,
            UpdateTask {
                title: Some("New Title".to_string()),
                description: Some("New description".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.title, "New Title");
    assert_eq!(updated.description, Some("New description".to_string()));
}

#[tokio::test]
async fn test_move_task_to_stage() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/project".to_string(),
        })
        .await
        .unwrap();

    let moved = repo.move_to_stage(&task.id, "in_progress").await.unwrap();

    assert_eq!(moved.stage, "in_progress");
}

#[tokio::test]
async fn test_delete_task() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo
        .create(CreateTask {
            title: "To Delete".to_string(),
            description: None,
            project_path: "/tmp/project".to_string(),
        })
        .await
        .unwrap();

    repo.delete(&task.id).await.unwrap();

    let result = repo.find(&task.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_task_not_found() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let result = repo.find("nonexistent").await;
    assert!(result.is_err());
}

// ==================== Task Model Tests ====================

#[test]
fn test_task_new() {
    let create = CreateTask {
        title: "Test Task".to_string(),
        description: Some("Description".to_string()),
        project_path: "/tmp/project".to_string(),
    };

    let task = Task::new(create);

    assert!(!task.id.is_empty());
    assert_eq!(task.title, "Test Task");
    assert_eq!(task.description, Some("Description".to_string()));
    assert_eq!(task.stage, "backlog");
    assert_eq!(task.project_path, "/tmp/project");
    assert_eq!(task.session_id, None);
    assert_eq!(task.priority, 0);
    assert_eq!(task.context, None);
    // Verify timestamps are recent
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(task.created_at);
    assert!(diff.num_seconds().abs() < 5);
}

#[test]
fn test_task_new_without_description() {
    let create = CreateTask {
        title: "No Description".to_string(),
        description: None,
        project_path: "/tmp/project".to_string(),
    };

    let task = Task::new(create);

    assert_eq!(task.description, None);
}

#[test]
fn test_update_task_default() {
    let update = UpdateTask::default();

    assert!(update.title.is_none());
    assert!(update.description.is_none());
    assert!(update.stage.is_none());
    assert!(update.priority.is_none());
    assert!(update.context.is_none());
}

// ==================== Stage Enum Tests ====================

#[test]
fn test_stage_as_str() {
    assert_eq!(Stage::Backlog.as_str(), "backlog");
    assert_eq!(Stage::Planning.as_str(), "planning");
    assert_eq!(Stage::Ready.as_str(), "ready");
    assert_eq!(Stage::InProgress.as_str(), "in_progress");
    assert_eq!(Stage::Review.as_str(), "review");
    assert_eq!(Stage::Done.as_str(), "done");
}

#[test]
fn test_stage_from_str_valid() {
    assert_eq!(Stage::from_str("backlog"), Some(Stage::Backlog));
    assert_eq!(Stage::from_str("planning"), Some(Stage::Planning));
    assert_eq!(Stage::from_str("ready"), Some(Stage::Ready));
    assert_eq!(Stage::from_str("in_progress"), Some(Stage::InProgress));
    assert_eq!(Stage::from_str("review"), Some(Stage::Review));
    assert_eq!(Stage::from_str("done"), Some(Stage::Done));
}

#[test]
fn test_stage_from_str_invalid() {
    assert_eq!(Stage::from_str("invalid"), None);
    assert_eq!(Stage::from_str("BACKLOG"), None); // Case sensitive
    assert_eq!(Stage::from_str(""), None);
    assert_eq!(Stage::from_str("back log"), None);
}

#[test]
fn test_stage_all() {
    let all = Stage::all();

    assert_eq!(all.len(), 6);
    assert!(all.contains(&"backlog"));
    assert!(all.contains(&"planning"));
    assert!(all.contains(&"ready"));
    assert!(all.contains(&"in_progress"));
    assert!(all.contains(&"review"));
    assert!(all.contains(&"done"));
}

#[test]
fn test_stage_roundtrip() {
    for stage_str in Stage::all() {
        let stage = Stage::from_str(stage_str).unwrap();
        assert_eq!(stage.as_str(), *stage_str);
    }
}

// ==================== CreateTask DTO Tests ====================

#[test]
fn test_create_task_serialization() {
    let create = CreateTask {
        title: "Test".to_string(),
        description: Some("Desc".to_string()),
        project_path: "/tmp/test".to_string(),
    };

    let json = serde_json::to_string(&create).unwrap();
    assert!(json.contains("\"title\":\"Test\""));
    assert!(json.contains("\"project_path\":\"/tmp/test\""));
}

#[test]
fn test_create_task_deserialization() {
    let json = r#"{"title":"Test","project_path":"/tmp/test"}"#;
    let create: CreateTask = serde_json::from_str(json).unwrap();

    assert_eq!(create.title, "Test");
    assert_eq!(create.description, None);
    assert_eq!(create.project_path, "/tmp/test");
}

// ==================== UpdateTask DTO Tests ====================

#[test]
fn test_update_task_serialization() {
    let update = UpdateTask {
        title: Some("New Title".to_string()),
        description: None,
        instructions: None,
        stage: Some("in_progress".to_string()),
        priority: Some(5),
        context: None,
        session_id: None,
    };

    let json = serde_json::to_string(&update).unwrap();
    assert!(json.contains("\"title\":\"New Title\""));
    assert!(json.contains("\"stage\":\"in_progress\""));
    assert!(json.contains("\"priority\":5"));
}

#[test]
fn test_update_task_deserialization_partial() {
    let json = r#"{"title":"Only Title"}"#;
    let update: UpdateTask = serde_json::from_str(json).unwrap();

    assert_eq!(update.title, Some("Only Title".to_string()));
    assert_eq!(update.description, None);
    assert_eq!(update.stage, None);
    assert_eq!(update.priority, None);
    assert_eq!(update.context, None);
}

// ==================== Task Serialization Tests ====================

#[test]
fn test_task_serialization() {
    let create = CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    };
    let task = Task::new(create);

    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"title\":\"Test\""));
    assert!(json.contains("\"stage\":\"backlog\""));
    assert!(json.contains("\"priority\":0"));
}

// ==================== Log Model Tests ====================

#[test]
fn test_log_new() {
    let create = CreateLog {
        level: "INFO".to_string(),
        message: "Test message".to_string(),
        target: Some("test_module".to_string()),
        source: Some("test_source".to_string()),
        task_id: Some("task-123".to_string()),
        session_id: Some("session-456".to_string()),
        metadata: Some(serde_json::json!({"key": "value"})),
    };

    let log = Log::new(create);

    assert_eq!(log.id, 0); // Will be set by database
    assert_eq!(log.level, "INFO");
    assert_eq!(log.message, "Test message");
    assert_eq!(log.target, Some("test_module".to_string()));
    assert_eq!(log.source, "test_source");
    assert_eq!(log.task_id, Some("task-123".to_string()));
    assert_eq!(log.session_id, Some("session-456".to_string()));
    assert!(log.metadata.is_some());
}

#[test]
fn test_log_new_with_defaults() {
    let create = CreateLog {
        level: "DEBUG".to_string(),
        message: "Simple log".to_string(),
        target: None,
        source: None,
        task_id: None,
        session_id: None,
        metadata: None,
    };

    let log = Log::new(create);

    assert_eq!(log.source, "backend"); // Default source
    assert_eq!(log.target, None);
    assert_eq!(log.metadata, None);
}

#[test]
fn test_log_filter_default() {
    let filter = LogFilter::default();

    assert!(filter.level.is_none());
    assert!(filter.source.is_none());
    assert!(filter.task_id.is_none());
    assert!(filter.session_id.is_none());
    assert!(filter.limit.is_none());
    assert!(filter.offset.is_none());
}

// ==================== level_to_str Helper Tests ====================

#[test]
fn test_level_to_str_standard() {
    use ai_kanban_backend::models::level_to_str;

    assert_eq!(level_to_str("debug"), "DEBUG");
    assert_eq!(level_to_str("info"), "INFO");
    assert_eq!(level_to_str("warn"), "WARN");
    assert_eq!(level_to_str("error"), "ERROR");
}

#[test]
fn test_level_to_str_case_insensitive() {
    use ai_kanban_backend::models::level_to_str;

    assert_eq!(level_to_str("DEBUG"), "DEBUG");
    assert_eq!(level_to_str("INFO"), "INFO");
    assert_eq!(level_to_str("WARN"), "WARN");
    assert_eq!(level_to_str("ERROR"), "ERROR");
    assert_eq!(level_to_str("Warning"), "WARN");
}

#[test]
fn test_level_to_str_unknown() {
    use ai_kanban_backend::models::level_to_str;

    assert_eq!(level_to_str("unknown"), "INFO");
    assert_eq!(level_to_str(""), "INFO");
    assert_eq!(level_to_str("critical"), "INFO");
}

// ==================== Task Repository Edge Cases ====================

#[tokio::test]
async fn test_task_repository_update_only_title() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo.create(CreateTask {
        title: "Original".to_string(),
        description: Some("Original desc".to_string()),
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let updated = repo.update(&task.id, UpdateTask {
        title: Some("New Title".to_string()),
        ..Default::default()
    }).await.unwrap();

    assert_eq!(updated.title, "New Title");
    assert_eq!(updated.description, Some("Original desc".to_string())); // Unchanged
    assert_eq!(updated.stage, "backlog"); // Unchanged
}

#[tokio::test]
async fn test_task_repository_update_only_priority() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let updated = repo.update(&task.id, UpdateTask {
        priority: Some(10),
        ..Default::default()
    }).await.unwrap();

    assert_eq!(updated.priority, 10);
    assert_eq!(updated.title, "Test"); // Unchanged
}

#[tokio::test]
async fn test_task_repository_update_only_stage() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let updated = repo.update(&task.id, UpdateTask {
        stage: Some("done".to_string()),
        ..Default::default()
    }).await.unwrap();

    assert_eq!(updated.stage, "done");
}

#[tokio::test]
async fn test_task_repository_update_nonexistent() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let result = repo.update("nonexistent-id", UpdateTask {
        title: Some("New Title".to_string()),
        ..Default::default()
    }).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_task_repository_delete_nonexistent() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let result = repo.delete("nonexistent-id").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_task_repository_list_ordering_by_priority() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    // Create tasks with different priorities (via update)
    let task1 = repo.create(CreateTask {
        title: "Low Priority".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let task2 = repo.create(CreateTask {
        title: "High Priority".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    repo.update(&task2.id, UpdateTask { priority: Some(10), ..Default::default() }).await.unwrap();
    repo.update(&task1.id, UpdateTask { priority: Some(1), ..Default::default() }).await.unwrap();

    let tasks = repo.list(None).await.unwrap();

    // Higher priority should come first
    assert_eq!(tasks[0].id, task2.id);
    assert_eq!(tasks[1].id, task1.id);
}

#[tokio::test]
async fn test_task_repository_move_records_history() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    // Move through multiple stages
    repo.move_to_stage(&task.id, "planning").await.unwrap();
    repo.move_to_stage(&task.id, "in_progress").await.unwrap();
    repo.move_to_stage(&task.id, "done").await.unwrap();

    let final_task = repo.find(&task.id).await.unwrap();
    assert_eq!(final_task.stage, "done");
}

#[tokio::test]
async fn test_task_repository_move_to_same_stage() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    // Move to same stage (should work)
    let result = repo.move_to_stage(&task.id, "backlog").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_repository_empty_description() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = repo.create(CreateTask {
        title: "No Description".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let found = repo.find(&task.id).await.unwrap();
    assert_eq!(found.description, None);
}

#[tokio::test]
async fn test_task_repository_context_field() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    // Create task without context
    let task = repo.create(CreateTask {
        title: "Test Context".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    // Verify context is None initially
    let found = repo.find(&task.id).await.unwrap();
    assert_eq!(found.context, None);

    // Update with context
    let markdown_context = "# Task Context\n\nThis is some markdown content.\n\n- Item 1\n- Item 2";
    let updated = repo.update(&task.id, UpdateTask {
        context: Some(markdown_context.to_string()),
        ..Default::default()
    }).await.unwrap();

    assert_eq!(updated.context, Some(markdown_context.to_string()));

    // Verify it persists
    let found_again = repo.find(&task.id).await.unwrap();
    assert_eq!(found_again.context, Some(markdown_context.to_string()));
}

#[tokio::test]
async fn test_task_repository_long_title() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let long_title = "a".repeat(1000);

    let task = repo.create(CreateTask {
        title: long_title.clone(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    assert_eq!(task.title.len(), 1000);
}

#[tokio::test]
async fn test_task_repository_special_characters() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    let special_title = "Test with emojis and \"quotes\" and 'apostrophes'";
    let special_desc = "Line1\nLine2\tTabbed";

    let task = repo.create(CreateTask {
        title: special_title.to_string(),
        description: Some(special_desc.to_string()),
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    let found = repo.find(&task.id).await.unwrap();
    assert_eq!(found.title, special_title);
    assert_eq!(found.description, Some(special_desc.to_string()));
}

#[tokio::test]
async fn test_task_repository_multiple_tasks_different_projects() {
    let (repo, _, _, _, _, _, _, _, _) = setup_test_db().await;

    repo.create(CreateTask {
        title: "Project A Task".to_string(),
        description: None,
        project_path: "/projects/a".to_string(),
    }).await.unwrap();

    repo.create(CreateTask {
        title: "Project B Task".to_string(),
        description: None,
        project_path: "/projects/b".to_string(),
    }).await.unwrap();

    let tasks = repo.list(None).await.unwrap();
    assert_eq!(tasks.len(), 2);
}

// ==================== Log Repository Edge Cases ====================

#[tokio::test]
async fn test_log_repository_multiple_levels() {
    let (_, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    for level in ["DEBUG", "INFO", "WARN", "ERROR"] {
        log_repo.create(CreateLog {
            level: level.to_string(),
            message: format!("{} message", level),
            target: None,
            source: None,
            task_id: None,
            session_id: None,
            metadata: None,
        }).await.unwrap();
    }

    let all_logs = log_repo.list(LogFilter::default()).await.unwrap();
    assert!(all_logs.len() >= 4);

    let error_logs = log_repo.list(LogFilter {
        level: Some("ERROR".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(error_logs.len(), 1);
}

#[tokio::test]
async fn test_log_repository_empty_message() {
    let (_, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    let log = log_repo.create(CreateLog {
        level: "INFO".to_string(),
        message: "".to_string(),
        target: None,
        source: None,
        task_id: None,
        session_id: None,
        metadata: None,
    }).await.unwrap();

    assert_eq!(log.message, "");
}

#[tokio::test]
async fn test_log_repository_long_message() {
    let (_, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    let long_message = "a".repeat(10000);

    let log = log_repo.create(CreateLog {
        level: "INFO".to_string(),
        message: long_message.clone(),
        target: None,
        source: None,
        task_id: None,
        session_id: None,
        metadata: None,
    }).await.unwrap();

    assert_eq!(log.message.len(), 10000);
}

#[tokio::test]
async fn test_log_repository_metadata_json() {
    let (_, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    let metadata = serde_json::json!({
        "user_id": 123,
        "action": "click",
        "details": {
            "button": "submit",
            "timestamp": "2024-01-01T00:00:00Z"
        }
    });

    let log = log_repo.create(CreateLog {
        level: "INFO".to_string(),
        message: "User action".to_string(),
        target: None,
        source: Some("frontend".to_string()),
        task_id: None,
        session_id: None,
        metadata: Some(metadata.clone()),
    }).await.unwrap();

    assert!(log.metadata.is_some());

    // Verify we can parse it back
    let parsed: serde_json::Value = serde_json::from_str(log.metadata.unwrap().trim_matches('"')).unwrap();
    assert_eq!(parsed["user_id"], 123);
}

#[tokio::test]
async fn test_log_repository_limit_boundary() {
    let (_, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    // Create 200 logs
    for i in 0..200 {
        log_repo.create(CreateLog {
            level: "INFO".to_string(),
            message: format!("Log {}", i),
            target: None,
            source: Some("frontend".to_string()),
            task_id: None,
            session_id: None,
            metadata: None,
        }).await.unwrap();
    }

    // Test limit is respected
    let logs = log_repo.list(LogFilter {
        limit: Some(50),
        source: Some("frontend".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(logs.len(), 50);

    // Test max limit (1000)
    let all_logs = log_repo.list(LogFilter {
        limit: Some(2000), // Request more than max
        source: Some("frontend".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(all_logs.len(), 200); // Should return all 200, not cap at 1000 since we have less
}

#[tokio::test]
async fn test_log_repository_offset_pagination() {
    let (_, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    // Create 10 logs
    for i in 0..10 {
        log_repo.create(CreateLog {
            level: "INFO".to_string(),
            message: format!("Log {}", i),
            target: None,
            source: Some("frontend".to_string()),
            task_id: None,
            session_id: None,
            metadata: None,
        }).await.unwrap();
    }

    // Get first page
    let page1 = log_repo.list(LogFilter {
        limit: Some(5),
        offset: Some(0),
        source: Some("frontend".to_string()),
        ..Default::default()
    }).await.unwrap();

    // Get second page
    let page2 = log_repo.list(LogFilter {
        limit: Some(5),
        offset: Some(5),
        source: Some("frontend".to_string()),
        ..Default::default()
    }).await.unwrap();

    // Pages should not overlap
    let ids1: std::collections::HashSet<_> = page1.iter().map(|l| l.id).collect();
    let ids2: std::collections::HashSet<_> = page2.iter().map(|l| l.id).collect();
    let intersection: std::collections::HashSet<_> = ids1.intersection(&ids2).collect();
    assert!(intersection.is_empty());
}

#[tokio::test]
async fn test_log_repository_combined_filters() {
    let (task_repo, log_repo, _, _, _, _, _, _, _) = setup_test_db().await;

    let task = task_repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    // Create logs with different combinations
    log_repo.create(CreateLog {
        level: "ERROR".to_string(),
        message: "Backend error".to_string(),
        target: None,
        source: Some("backend".to_string()),
        task_id: Some(task.id.clone()),
        session_id: Some("s1".to_string()),
        metadata: None,
    }).await.unwrap();

    log_repo.create(CreateLog {
        level: "INFO".to_string(),
        message: "Frontend info".to_string(),
        target: None,
        source: Some("frontend".to_string()),
        task_id: Some(task.id.clone()),
        session_id: Some("s1".to_string()),
        metadata: None,
    }).await.unwrap();

    log_repo.create(CreateLog {
        level: "ERROR".to_string(),
        message: "Other error".to_string(),
        target: None,
        source: Some("backend".to_string()),
        task_id: None,
        session_id: None,
        metadata: None,
    }).await.unwrap();

    // Filter by task + level
    let filtered = log_repo.list(LogFilter {
        task_id: Some(task.id.clone()),
        level: Some("ERROR".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].message, "Backend error");
}

// ==================== AppState Tests ====================

#[tokio::test]
async fn test_app_state_new() {
    let (task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo) = setup_test_db().await;

    let state = AppState::new(task_repo.clone(), log_repo.clone(), session_repo.clone(), comment_repo.clone(), token_event_repo.clone(), session_metrics_repo.clone(), settings_repo, otel_metrics_repo, attachment_repo);

    // Verify the state is created (repositories are Clone)
    let _ = state.tasks;
    let _ = state.logs;
    let _ = state.sessions;
    let _ = state.comments;
}

#[tokio::test]
async fn test_app_state_into_task_api_state() {
    let (task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo) = setup_test_db().await;

    let state = AppState::new(task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo);
    let task_api_state: TaskApiState = state.into();

    // Verify we can use the task repository
    let task = task_api_state.repo.create(CreateTask {
        title: "Test".to_string(),
        description: None,
        project_path: "/tmp/test".to_string(),
    }).await.unwrap();

    assert!(!task.id.is_empty());
}

#[tokio::test]
async fn test_app_state_into_log_api_state() {
    let (task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo) = setup_test_db().await;

    let state = AppState::new(task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo);
    let log_api_state: LogApiState = state.into();

    // Verify we can use the log repository
    let log = log_api_state.repo.create(CreateLog {
        level: "INFO".to_string(),
        message: "Test".to_string(),
        target: None,
        source: None,
        task_id: None,
        session_id: None,
        metadata: None,
    }).await.unwrap();

    assert!(log.id > 0);
}

#[tokio::test]
async fn test_app_state_into_session_api_state() {
    use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
    use std::sync::Arc;

    let (task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo) = setup_test_db().await;

    let manager = Arc::new(ClaudeManager::new(
        session_repo.clone(), token_event_repo.clone(), session_metrics_repo.clone(),
        comment_repo.clone(), task_repo.clone(), otel_metrics_repo.clone(), None, None,
        attachment_repo.clone(),
    ));
    let queue = Arc::new(SessionQueue::new(manager, task_repo.clone()));
    let state = AppState::new(task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo).with_queue(queue);
    let session_api_state: SessionApiState = state.into();

    // Verify the queue is accessible
    let _ = &session_api_state.queue;
}
