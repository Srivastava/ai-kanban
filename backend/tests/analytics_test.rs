use ai_kanban_backend::db::{
    AnalyticsRepository, SessionRepository, TaskRepository, TokenEventRepository, create_pool,
};
use ai_kanban_backend::models::{CreateSession, CreateTask, CreateTokenEvent};

async fn setup_db() -> sqlx::SqlitePool {
    create_pool(":memory:").await.expect("pool")
}

/// Seed a single token event with a task in the given stage.
async fn seed_one_event(
    pool: &sqlx::SqlitePool,
    stage: &str,
    tool_name: Option<&str>,
    input: i64,
    output: i64,
) -> (String, String) {
    let task_repo = TaskRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let event_repo = TokenEventRepository::new(pool.clone());

    let task = task_repo
        .create(CreateTask {
            title: format!("Task-{}", stage),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    sqlx::query("UPDATE tasks SET stage = ? WHERE id = ?")
        .bind(stage)
        .bind(&task.id)
        .execute(pool)
        .await
        .unwrap();

    let session = session_repo
        .create(CreateSession { task_id: task.id.clone() })
        .await
        .unwrap();

    event_repo
        .create(CreateTokenEvent {
            session_id: session.id.clone(),
            task_id: task.id.clone(),
            event_type: "assistant".to_string(),
            tool_name: tool_name.map(str::to_string),
            file_ext: None,
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: None,
            sequence_no: Some(0),
        })
        .await
        .unwrap();

    (task.id, session.id)
}

#[tokio::test]
async fn test_cost_by_task_empty() {
    let pool = setup_db().await;
    let repo = AnalyticsRepository::new(pool);
    let rows = repo.cost_by_task().await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_cost_by_task_calculates_cost_correctly() {
    let pool = setup_db().await;
    seed_one_event(&pool, "in_progress", None, 1_000_000, 1_000_000).await;
    let repo = AnalyticsRepository::new(pool);
    let rows = repo.cost_by_task().await.unwrap();
    assert_eq!(rows.len(), 1);
    // 1M input * $3/1M + 1M output * $15/1M = $18.00
    assert!((rows[0].cost_usd - 18.0_f64).abs() < 0.001);
}

#[tokio::test]
async fn test_cost_by_task_ordered_desc() {
    let pool = setup_db().await;
    seed_one_event(&pool, "backlog", None, 100, 10).await;
    seed_one_event(&pool, "in_progress", None, 900_000, 300_000).await;
    let repo = AnalyticsRepository::new(pool);
    let rows = repo.cost_by_task().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].cost_usd >= rows[1].cost_usd);
}

#[tokio::test]
async fn test_tokens_by_stage_empty() {
    let pool = setup_db().await;
    let repo = AnalyticsRepository::new(pool);
    assert!(repo.tokens_by_stage().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_tokens_by_stage_groups_correctly() {
    let pool = setup_db().await;
    seed_one_event(&pool, "backlog", None, 100, 20).await;
    seed_one_event(&pool, "backlog", None, 200, 40).await;
    seed_one_event(&pool, "done", None, 500, 100).await;
    let repo = AnalyticsRepository::new(pool);
    let rows = repo.tokens_by_stage().await.unwrap();
    assert_eq!(rows.len(), 2);
    let backlog = rows.iter().find(|r| r.stage == "backlog").unwrap();
    assert_eq!(backlog.input_tokens, 300);
    let done = rows.iter().find(|r| r.stage == "done").unwrap();
    assert_eq!(done.input_tokens, 500);
}

#[tokio::test]
async fn test_session_summary_zero_sessions() {
    let pool = setup_db().await;
    let summary = AnalyticsRepository::new(pool).session_summary().await.unwrap();
    assert_eq!(summary.total_sessions, 0);
    assert_eq!(summary.total_cost_usd, 0.0);
}

#[tokio::test]
async fn test_session_summary_aggregates() {
    let pool = setup_db().await;
    seed_one_event(&pool, "backlog", None, 1000, 0).await;
    seed_one_event(&pool, "backlog", None, 0, 500).await;
    let summary = AnalyticsRepository::new(pool).session_summary().await.unwrap();
    assert_eq!(summary.total_sessions, 2);
    assert_eq!(summary.max_tokens_per_session, 1000);
    assert!((summary.avg_tokens_per_session - 750.0).abs() < 0.01);
}

#[tokio::test]
async fn test_burn_rate_no_recent_events() {
    let pool = setup_db().await;
    let rate = AnalyticsRepository::new(pool).burn_rate().await.unwrap();
    assert_eq!(rate.tokens_last_hour, 0.0);
}

#[tokio::test]
async fn test_burn_rate_counts_recent_events() {
    let pool = setup_db().await;
    seed_one_event(&pool, "in_progress", None, 600, 0).await;
    let rate = AnalyticsRepository::new(pool).burn_rate().await.unwrap();
    assert!(rate.tokens_last_hour >= 600.0);
    assert!(rate.tokens_per_minute >= 0.0);
}
