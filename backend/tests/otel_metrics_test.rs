use ai_kanban_backend::db::{create_pool, OtelMetricsRepository, SessionRepository, TaskRepository};
use ai_kanban_backend::models::{CreateOtelMetric, CreateSession, CreateTask};

async fn setup() -> (OtelMetricsRepository, SessionRepository, TaskRepository) {
    let db_path = format!("/tmp/test-otel-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    (
        OtelMetricsRepository::new(pool.clone()),
        SessionRepository::new(pool.clone()),
        TaskRepository::new(pool.clone()),
    )
}

#[tokio::test]
async fn test_insert_and_query_unaffiliated() {
    let (repo, _, _) = setup().await;
    repo.insert(CreateOtelMetric {
        metric_name: "claude_code.token.usage".to_string(),
        value: 1000.0,
        unit: Some("token".to_string()),
        session_id: None,
        task_id: None,
        claude_session_id: "external-session-abc".to_string(),
        attributes: serde_json::json!({"type": "input"}),
        otel_timestamp: 1709000000000000000,
    }).await.unwrap();

    let rows = repo.dev_activity().await.unwrap();
    // Unaffiliated session (no task_id) should NOT appear in dev_activity
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_dev_activity_correlated_session() {
    let (repo, session_repo, task_repo) = setup().await;

    let task = task_repo.create(CreateTask {
        title: "Dev Task".to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    repo.insert(CreateOtelMetric {
        metric_name: "claude_code.commit.count".to_string(),
        value: 3.0,
        unit: None,
        session_id: Some(session.id.clone()),
        task_id: Some(task.id.clone()),
        claude_session_id: "acto-session-xyz".to_string(),
        attributes: serde_json::json!({}),
        otel_timestamp: 1709000000000000000,
    }).await.unwrap();

    let rows = repo.dev_activity().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].task_title, "Dev Task");
    assert!((rows[0].commits - 3.0).abs() < 0.01);
}

#[tokio::test]
async fn test_correlate_by_claude_session_id() {
    let (repo, session_repo, task_repo) = setup().await;

    let task = task_repo.create(CreateTask {
        title: "Corr Task".to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    }).await.unwrap();

    let session = session_repo.create(CreateSession { task_id: task.id.clone() }).await.unwrap();

    let claude_sid = "known-claude-session-id";
    repo.insert(CreateOtelMetric {
        metric_name: "claude_code.active_time.total".to_string(),
        value: 3600.0,
        unit: Some("s".to_string()),
        session_id: None,
        task_id: None,
        claude_session_id: claude_sid.to_string(),
        attributes: serde_json::json!({}),
        otel_timestamp: 1709000000000000000,
    }).await.unwrap();

    // Correlate after the fact
    repo.correlate(claude_sid, &session.id, &task.id).await.unwrap();

    let rows = repo.dev_activity().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!((rows[0].active_time_secs - 3600.0).abs() < 0.01);
}
