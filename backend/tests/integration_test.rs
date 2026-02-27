use ai_kanban_backend::db::{create_pool, TaskRepository};
use ai_kanban_backend::models::{CreateLog, CreateTask, Log, LogFilter, Stage, Task, UpdateTask};

async fn setup_test_db() -> TaskRepository {
    let db_path = format!("/tmp/test-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    TaskRepository::new(pool)
}

#[tokio::test]
async fn test_create_task() {
    let repo = setup_test_db().await;

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
    let repo = setup_test_db().await;

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
    let repo = setup_test_db().await;

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
    let repo = setup_test_db().await;

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
    let repo = setup_test_db().await;

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
    let repo = setup_test_db().await;

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
    let repo = setup_test_db().await;

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
        stage: Some("in_progress".to_string()),
        priority: Some(5),
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
