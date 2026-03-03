use ai_kanban_backend::db::{create_pool, SessionMetricsRepository, TokenEventRepository, TaskRepository, SessionRepository};
use ai_kanban_backend::models::{CreateTokenEvent, CreateTask, CreateSession};

async fn setup_db() -> sqlx::SqlitePool {
    create_pool(":memory:").await.expect("Failed to create test pool")
}

async fn create_parent_rows(pool: &sqlx::SqlitePool) -> (String, String) {
    let task_repo = TaskRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());

    let task = task_repo
        .create(CreateTask {
            title: "Test task".to_string(),
            description: None,
            project_path: "/test".to_string(),
        })
        .await
        .expect("Failed to create task");

    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .expect("Failed to create session");

    (task.id, session.id)
}

#[tokio::test]
async fn test_create_token_event() {
    let pool = setup_db().await;
    let repo = TokenEventRepository::new(pool.clone());
    let (task_id, session_id) = create_parent_rows(&pool).await;

    let event = repo
        .create(CreateTokenEvent {
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            event_type: "assistant".to_string(),
            tool_name: Some("Read".to_string()),
            file_ext: Some(".rs".to_string()),
            input_tokens: 100,
            output_tokens: 50,
            model: Some("claude-sonnet-4-6".to_string()),
            sequence_no: Some(0),
        })
        .await
        .expect("Failed to create token event");

    assert_eq!(event.session_id, session_id);
    assert_eq!(event.input_tokens, 100);
    assert_eq!(event.tool_name, Some("Read".to_string()));
}

#[tokio::test]
async fn test_list_by_session() {
    let pool = setup_db().await;
    let repo = TokenEventRepository::new(pool.clone());
    let (task_id, session_id_a) = create_parent_rows(&pool).await;

    repo.create(CreateTokenEvent {
        session_id: session_id_a.clone(),
        task_id: task_id.clone(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 100,
        output_tokens: 20,
        model: None,
        sequence_no: Some(0),
    })
    .await
    .unwrap();

    repo.create(CreateTokenEvent {
        session_id: session_id_a.clone(),
        task_id: task_id.clone(),
        event_type: "result".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 200,
        output_tokens: 80,
        model: None,
        sequence_no: Some(1),
    })
    .await
    .unwrap();

    // Create second session for comparison
    let session_repo = SessionRepository::new(pool.clone());
    let session_b = session_repo.create(CreateSession { task_id: task_id.clone() }).await.unwrap();

    repo.create(CreateTokenEvent {
        session_id: session_b.id.clone(),
        task_id: task_id.clone(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 50,
        output_tokens: 10,
        model: None,
        sequence_no: Some(0),
    })
    .await
    .unwrap();

    let events = repo.list_by_session(&session_id_a).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence_no, 0);
    assert_eq!(events[1].sequence_no, 1);

    let other = repo.list_by_session(&session_b.id).await.unwrap();
    assert_eq!(other.len(), 1);
}

#[tokio::test]
async fn test_upsert_session_metrics() {
    let pool = setup_db().await;
    let repo = SessionMetricsRepository::new(pool.clone());
    let (_, session_id) = create_parent_rows(&pool).await;

    repo.upsert(&session_id, 42, 1500).await.unwrap();

    let m = repo.find(&session_id).await.unwrap().unwrap();
    assert_eq!(m.project_files, 42);
    assert_eq!(m.project_loc, 1500);
    assert_eq!(m.lines_written, 0);

    repo.add_lines_written(&session_id, 10).await.unwrap();
    let m2 = repo.find(&session_id).await.unwrap().unwrap();
    assert_eq!(m2.lines_written, 10);

    repo.add_lines_written(&session_id, 5).await.unwrap();
    let m3 = repo.find(&session_id).await.unwrap().unwrap();
    assert_eq!(m3.lines_written, 15);
}

#[tokio::test]
async fn test_list_by_task() {
    let pool = setup_db().await;
    let repo = TokenEventRepository::new(pool.clone());
    let (task_id_a, session_id_a) = create_parent_rows(&pool).await;
    let (task_id_b, session_id_b) = create_parent_rows(&pool).await;

    repo.create(CreateTokenEvent {
        session_id: session_id_a.clone(),
        task_id: task_id_a.clone(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 10,
        output_tokens: 5,
        model: None,
        sequence_no: Some(0),
    }).await.unwrap();

    repo.create(CreateTokenEvent {
        session_id: session_id_b.clone(),
        task_id: task_id_b.clone(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: 20,
        output_tokens: 10,
        model: None,
        sequence_no: Some(0),
    }).await.unwrap();

    let events = repo.list_by_task(&task_id_a).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].task_id, task_id_a);
}

#[tokio::test]
async fn test_create_batch() {
    let pool = setup_db().await;
    let repo = TokenEventRepository::new(pool.clone());
    let (task_id, session_id) = create_parent_rows(&pool).await;

    let events = vec![
        CreateTokenEvent {
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            event_type: "assistant".to_string(),
            tool_name: Some("Read".to_string()),
            file_ext: Some(".rs".to_string()),
            input_tokens: 100,
            output_tokens: 50,
            model: Some("claude-sonnet".to_string()),
            sequence_no: Some(0),
        },
        CreateTokenEvent {
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            event_type: "result".to_string(),
            tool_name: None,
            file_ext: None,
            input_tokens: 200,
            output_tokens: 100,
            model: None,
            sequence_no: Some(1),
        },
    ];

    repo.create_batch(events).await.unwrap();

    let stored = repo.list_by_session(&session_id).await.unwrap();
    assert_eq!(stored.len(), 2);
}

#[tokio::test]
async fn test_add_lines_deleted() {
    let pool = setup_db().await;
    let metrics_repo = SessionMetricsRepository::new(pool.clone());
    let (_, session_id) = create_parent_rows(&pool).await;

    metrics_repo.upsert(&session_id, 10, 1000).await.unwrap();
    metrics_repo.add_lines_deleted(&session_id, 5).await.unwrap();

    let metrics = metrics_repo.find(&session_id).await.unwrap().unwrap();
    assert_eq!(metrics.lines_deleted, 5);
}
