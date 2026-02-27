use ai_kanban_backend::db::{create_pool, TaskRepository};
use ai_kanban_backend::models::{CreateTask, Stage, UpdateTask};

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

#[tokio::test]
async fn test_stage_enum() {
    assert_eq!(Stage::Backlog.as_str(), "backlog");
    assert_eq!(Stage::InProgress.as_str(), "in_progress");

    assert_eq!(Stage::from_str("backlog"), Some(Stage::Backlog));
    assert_eq!(Stage::from_str("in_progress"), Some(Stage::InProgress));
    assert_eq!(Stage::from_str("invalid"), None);

    assert_eq!(Stage::all().len(), 6);
}
