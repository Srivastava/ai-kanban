use ai_kanban_backend::db::{
    create_pool, AnalyticsRepository, OtelMetricsRepository, SessionMetricsRepository,
    SessionRepository, TaskRepository, TokenEventRepository,
};
use ai_kanban_backend::models::{CreateOtelMetric, CreateSession, CreateTask, CreateTokenEvent};

struct Repos {
    analytics: AnalyticsRepository,
    token_repo: TokenEventRepository,
    task_repo: TaskRepository,
    session_repo: SessionRepository,
    metrics_repo: SessionMetricsRepository,
}

async fn setup() -> Repos {
    let db_path = format!("/tmp/test-analytics-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    Repos {
        analytics: AnalyticsRepository::new(pool.clone()),
        token_repo: TokenEventRepository::new(pool.clone()),
        task_repo: TaskRepository::new(pool.clone()),
        session_repo: SessionRepository::new(pool.clone()),
        metrics_repo: SessionMetricsRepository::new(pool.clone()),
    }
}

/// Create a task and a session, return (task_id, session_id)
async fn create_task_and_session(repos: &Repos) -> (String, String) {
    let task = repos
        .task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();

    let session = repos
        .session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();

    (task.id, session.id)
}

fn make_event(
    session_id: &str,
    task_id: &str,
    input: i64,
    output: i64,
    seq: i64,
) -> CreateTokenEvent {
    CreateTokenEvent {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        event_type: "assistant".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        model: None,
        sequence_no: Some(seq),
    }
}

// --- overview ---

#[tokio::test]
async fn test_overview_empty() {
    let repos = setup().await;
    let overview = repos.analytics.overview().await.unwrap();
    assert_eq!(overview.total_input_tokens, 0);
    assert_eq!(overview.total_output_tokens, 0);
    assert_eq!(overview.total_sessions, 0);
    assert_eq!(overview.total_tasks_with_sessions, 0);
    assert_eq!(overview.estimated_cost_usd, 0.0);
}

#[tokio::test]
async fn test_overview_with_data() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 1_000_000, 1_000_000, 0))
        .await
        .unwrap();
    let overview = repos.analytics.overview().await.unwrap();
    assert_eq!(overview.total_input_tokens, 1_000_000);
    assert_eq!(overview.total_output_tokens, 1_000_000);
    // cost = 1.0 * 3 + 1.0 * 15 = 18.0
    assert!(
        (overview.estimated_cost_usd - 18.0).abs() < 0.01,
        "expected ~18.0, got {}",
        overview.estimated_cost_usd
    );
    assert_eq!(overview.total_sessions, 1);
    assert_eq!(overview.total_tasks_with_sessions, 1);
}

// --- daily_tokens ---

#[tokio::test]
async fn test_daily_tokens_empty() {
    let repos = setup().await;
    let rows = repos.analytics.daily_tokens(7, None).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_daily_tokens_today() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 100, 50, 0))
        .await
        .unwrap();
    let rows = repos.analytics.daily_tokens(7, None).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].input_tokens, 100);
    assert_eq!(rows[0].output_tokens, 50);
}

// --- weekly_tokens ---

#[tokio::test]
async fn test_weekly_tokens_empty() {
    let repos = setup().await;
    let rows = repos.analytics.weekly_tokens(4, None).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_weekly_tokens_with_data() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 200, 100, 0))
        .await
        .unwrap();
    let rows = repos.analytics.weekly_tokens(4, None).await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].input_tokens, 200);
}

// --- monthly_tokens ---

#[tokio::test]
async fn test_monthly_tokens_empty() {
    let repos = setup().await;
    let rows = repos.analytics.monthly_tokens(3, None).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_monthly_tokens_with_data() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 500, 250, 0))
        .await
        .unwrap();
    let rows = repos.analytics.monthly_tokens(3, None).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].input_tokens, 500);
}

// --- tokens_by_task ---

#[tokio::test]
async fn test_tokens_by_task_empty() {
    let repos = setup().await;
    let rows = repos.analytics.tokens_by_task().await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_tokens_by_task_aggregates() {
    let repos = setup().await;

    // Create a named task and two sessions for it
    let task = repos
        .task_repo
        .create(CreateTask {
            title: "My Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();
    let session1 = repos
        .session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    let session2 = repos
        .session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();

    repos
        .token_repo
        .create(make_event(&session1.id, &task.id, 100, 50, 0))
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session2.id, &task.id, 200, 100, 0))
        .await
        .unwrap();

    let rows = repos.analytics.tokens_by_task().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].input_tokens, 300);
    assert_eq!(rows[0].output_tokens, 150);
    assert_eq!(rows[0].total_tokens, 450);
    assert_eq!(rows[0].task_title, "My Task");
}

// --- tokens_by_session ---

#[tokio::test]
async fn test_tokens_by_session_empty() {
    let repos = setup().await;
    let rows = repos.analytics.tokens_by_session().await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_tokens_by_session_groups_by_session() {
    let repos = setup().await;

    // Create a task and two sessions
    let task = repos
        .task_repo
        .create(CreateTask {
            title: "Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();
    let session1 = repos
        .session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();
    let session2 = repos
        .session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();

    repos
        .token_repo
        .create(make_event(&session1.id, &task.id, 100, 50, 0))
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session1.id, &task.id, 50, 25, 1))
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session2.id, &task.id, 200, 100, 0))
        .await
        .unwrap();

    let rows = repos.analytics.tokens_by_session().await.unwrap();
    assert_eq!(rows.len(), 2);
    let s1 = rows.iter().find(|r| r.session_id == session1.id).unwrap();
    assert_eq!(s1.input_tokens, 150);
    assert_eq!(s1.total_tokens, 225);
}

// --- tokens_by_tool ---

#[tokio::test]
async fn test_tokens_by_tool_empty() {
    let repos = setup().await;
    let rows = repos.analytics.tokens_by_tool(None).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_tokens_by_tool_groups_correctly() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;

    let mut ev = make_event(&session_id, &task_id, 100, 50, 0);
    ev.tool_name = Some("Read".to_string());
    repos.token_repo.create(ev).await.unwrap();

    let mut ev2 = make_event(&session_id, &task_id, 200, 100, 1);
    ev2.tool_name = Some("Read".to_string());
    repos.token_repo.create(ev2).await.unwrap();

    let mut ev3 = make_event(&session_id, &task_id, 50, 25, 2);
    ev3.tool_name = Some("Write".to_string());
    repos.token_repo.create(ev3).await.unwrap();

    // event with no tool_name should not appear in tokens_by_tool
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 10, 5, 3))
        .await
        .unwrap();

    let rows = repos.analytics.tokens_by_tool(None).await.unwrap();
    assert_eq!(rows.len(), 2);
    let read = rows.iter().find(|r| r.tool_name == "Read").unwrap();
    assert_eq!(read.call_count, 2);
    assert_eq!(read.input_tokens, 300);
}

// --- tokens_by_language ---

#[tokio::test]
async fn test_tokens_by_language_empty() {
    let repos = setup().await;
    let rows = repos.analytics.tokens_by_language(None).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_tokens_by_language_groups_by_ext() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;

    let mut ev = make_event(&session_id, &task_id, 100, 50, 0);
    ev.file_ext = Some(".rs".to_string());
    repos.token_repo.create(ev).await.unwrap();

    let mut ev2 = make_event(&session_id, &task_id, 200, 100, 1);
    ev2.file_ext = Some(".ts".to_string());
    repos.token_repo.create(ev2).await.unwrap();

    let mut ev3 = make_event(&session_id, &task_id, 50, 25, 2);
    ev3.file_ext = Some(".rs".to_string());
    repos.token_repo.create(ev3).await.unwrap();

    let rows = repos.analytics.tokens_by_language(None).await.unwrap();
    assert_eq!(rows.len(), 2);
    let rs = rows.iter().find(|r| r.file_ext == ".rs").unwrap();
    assert_eq!(rs.call_count, 2);
    assert_eq!(rs.input_tokens, 150);
}

// --- token_efficiency ---

#[tokio::test]
async fn test_token_efficiency_empty() {
    let repos = setup().await;
    let rows = repos.analytics.token_efficiency(None).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn test_token_efficiency_no_lines_written() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 1000, 500, 0))
        .await
        .unwrap();

    let rows = repos.analytics.token_efficiency(None).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].total_tokens, 1500);
    assert!(rows[0].tokens_per_line.is_none()); // no lines written
}

#[tokio::test]
async fn test_token_efficiency_with_metrics() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;

    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 1000, 500, 0))
        .await
        .unwrap();
    // Efficiency computes MAX(project_loc) - MIN(project_loc) joined via token_events.
    // Need a second session with a token event and higher project_loc to get a non-zero delta.
    let session_b = repos
        .session_repo
        .create(ai_kanban_backend::models::CreateSession {
            task_id: task_id.clone(),
        })
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session_b.id, &task_id, 0, 0, 1))
        .await
        .unwrap();
    repos
        .metrics_repo
        .upsert(&session_id, 10, 100)
        .await
        .unwrap(); // starting LOC
    repos
        .metrics_repo
        .upsert(&session_b.id, 10, 200)
        .await
        .unwrap(); // ending LOC, delta=100

    let rows = repos.analytics.token_efficiency(None).await.unwrap();
    let our_row = rows.iter().find(|r| r.total_tokens == 1500).unwrap();
    assert!(our_row.tokens_per_line.is_some());
    let tpl = our_row.tokens_per_line.unwrap();
    // 1500 tokens / 100 LOC-delta = 15.0
    assert!((tpl - 15.0).abs() < 0.01, "expected ~15.0, got {}", tpl);
}

// --- usage_windows ---

#[tokio::test]
async fn test_usage_windows_empty() {
    let repos = setup().await;
    let w = repos
        .analytics
        .usage_windows(50_000, 1_000_000)
        .await
        .unwrap();
    assert_eq!(w.tokens_5hr, 0);
    assert_eq!(w.tokens_week, 0);
    assert_eq!(w.limit_5hr, 50_000);
    assert_eq!(w.limit_week, 1_000_000);
    assert!(w.reset_5hr.is_none());
}

#[tokio::test]
async fn test_usage_windows_with_recent_data() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 1000, 500, 0))
        .await
        .unwrap();
    let w = repos
        .analytics
        .usage_windows(50_000, 1_000_000)
        .await
        .unwrap();
    assert_eq!(w.tokens_5hr, 1500);
    assert_eq!(w.tokens_week, 1500);
    assert!(w.reset_5hr.is_some());
}

// --- session_timeline ---

#[tokio::test]
async fn test_session_timeline_empty() {
    let repos = setup().await;
    let events = repos
        .analytics
        .session_timeline("nonexistent-session")
        .await
        .unwrap();
    assert!(events.is_empty());
}

#[tokio::test]
async fn test_session_timeline_cumulative_totals() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 100, 50, 0))
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 200, 100, 1))
        .await
        .unwrap();

    let events = repos.analytics.session_timeline(&session_id).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].cumulative_total, 150); // 100+50
    assert_eq!(events[1].cumulative_total, 450); // 150 + 200+100
}

#[tokio::test]
async fn test_session_timeline_ordered_by_sequence() {
    let repos = setup().await;
    let (task_id, session_id) = create_task_and_session(&repos).await;

    // Insert out of order
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 50, 25, 2))
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 100, 50, 0))
        .await
        .unwrap();
    repos
        .token_repo
        .create(make_event(&session_id, &task_id, 200, 100, 1))
        .await
        .unwrap();

    let events = repos.analytics.session_timeline(&session_id).await.unwrap();
    // sequence_no is i64 (not Option<i64>) per SessionTimelineEvent definition
    assert_eq!(events[0].sequence_no, 0);
    assert_eq!(events[1].sequence_no, 1);
    assert_eq!(events[2].sequence_no, 2);
}

async fn setup_roi_db() -> (
    sqlx::SqlitePool,
    TaskRepository,
    SessionRepository,
    TokenEventRepository,
    OtelMetricsRepository,
    AnalyticsRepository,
) {
    let pool = create_pool(":memory:").await.unwrap();
    (
        pool.clone(),
        TaskRepository::new(pool.clone()),
        SessionRepository::new(pool.clone()),
        TokenEventRepository::new(pool.clone()),
        OtelMetricsRepository::new(pool.clone()),
        AnalyticsRepository::new(pool.clone()),
    )
}

#[tokio::test]
async fn test_roi_metrics_no_data() {
    let (_, _, _, _, _, analytics) = setup_roi_db().await;
    let roi = analytics.roi_metrics(None).await.unwrap();
    assert_eq!(roi.total_commits, 0);
    assert_eq!(roi.total_prs, 0);
    assert!(roi.cost_per_commit.is_none());
    assert!(roi.cost_per_pr.is_none());
    assert_eq!(roi.total_cost_usd, 0.0);
}

#[tokio::test]
async fn test_roi_metrics_with_data() {
    let (_, task_repo, session_repo, event_repo, otel_repo, analytics) = setup_roi_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();

    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();

    // Seed token event (100K input, 50K output)
    event_repo
        .create(CreateTokenEvent {
            session_id: session.id.clone(),
            task_id: task.id.clone(),
            event_type: "assistant".to_string(),
            tool_name: None,
            file_ext: None,
            input_tokens: 100_000,
            output_tokens: 50_000,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            model: None,
            sequence_no: Some(0),
        })
        .await
        .unwrap();

    // Seed commit + PR OTel metrics
    for (name, val) in [
        ("claude_code.commit.count", 3.0),
        ("claude_code.pull_request.count", 1.0),
    ] {
        otel_repo
            .insert(CreateOtelMetric {
                metric_name: name.to_string(),
                value: val,
                unit: None,
                session_id: Some(session.id.clone()),
                task_id: Some(task.id.clone()),
                claude_session_id: "cs-abc".to_string(),
                attributes: serde_json::json!({}),
                otel_timestamp: 1_709_000_000_000_000_000,
            })
            .await
            .unwrap();
    }

    let roi = analytics.roi_metrics(None).await.unwrap();
    assert_eq!(roi.total_commits, 3);
    assert_eq!(roi.total_prs, 1);
    assert!(roi.cost_per_commit.is_some());
    assert!(roi.cost_per_pr.is_some());
    // 100K input @ $3/M = $0.30; 50K output @ $15/M = $0.75; total = $1.05
    assert!(
        (roi.total_cost_usd - 1.05).abs() < 0.01,
        "expected ~1.05, got {}",
        roi.total_cost_usd
    );
    // cost_per_commit = 1.05 / 3 ≈ 0.35
    assert!((roi.cost_per_commit.unwrap() - 0.35).abs() < 0.01);
}

#[tokio::test]
async fn test_roi_metrics_task_filter() {
    let (_, task_repo, session_repo, _, otel_repo, analytics) = setup_roi_db().await;

    // Create two tasks
    let t1 = task_repo
        .create(CreateTask {
            title: "T1".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();
    let t2 = task_repo
        .create(CreateTask {
            title: "T2".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();
    let s1 = session_repo
        .create(CreateSession {
            task_id: t1.id.clone(),
        })
        .await
        .unwrap();
    let s2 = session_repo
        .create(CreateSession {
            task_id: t2.id.clone(),
        })
        .await
        .unwrap();

    for (tid, sid, commits) in [(&t1.id, &s1.id, 2.0), (&t2.id, &s2.id, 5.0)] {
        otel_repo
            .insert(CreateOtelMetric {
                metric_name: "claude_code.commit.count".to_string(),
                value: commits,
                unit: None,
                session_id: Some(sid.clone()),
                task_id: Some(tid.clone()),
                claude_session_id: "cs-x".to_string(),
                attributes: serde_json::json!({}),
                otel_timestamp: 1_709_000_000_000_000_000,
            })
            .await
            .unwrap();
    }

    let all = analytics.roi_metrics(None).await.unwrap();
    assert_eq!(all.total_commits, 7); // 2 + 5

    let filtered = analytics.roi_metrics(Some(&t1.id)).await.unwrap();
    assert_eq!(filtered.total_commits, 2);
}

#[tokio::test]
async fn test_context_window_usage_empty_when_no_running_sessions() {
    let (_, _, _, _, _, analytics) = setup_roi_db().await;
    let result = analytics.context_window_usage().await.unwrap();
    assert!(result.is_empty(), "no running sessions → empty result");
}

#[tokio::test]
async fn test_context_window_usage_running_session() {
    let (pool, task_repo, session_repo, event_repo, _, analytics) = setup_roi_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Running Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .unwrap();

    let session = session_repo
        .create(CreateSession {
            task_id: task.id.clone(),
        })
        .await
        .unwrap();

    // Mark session as running
    sqlx::query("UPDATE sessions SET status = 'running' WHERE id = ?")
        .bind(&session.id)
        .execute(&pool)
        .await
        .unwrap();

    // Add token event: 50K input, 30K cache_read, 5K cache_creation
    event_repo
        .create(CreateTokenEvent {
            session_id: session.id.clone(),
            task_id: task.id.clone(),
            event_type: "assistant".to_string(),
            tool_name: None,
            file_ext: None,
            input_tokens: 50_000,
            output_tokens: 10_000,
            cache_read_tokens: 30_000,
            cache_creation_tokens: 5_000,
            model: None,
            sequence_no: Some(0),
        })
        .await
        .unwrap();

    let result = analytics.context_window_usage().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].task_title, "Running Task");
    // tokens_in_window = 50_000 + 30_000 + 5_000 = 85_000
    assert_eq!(result[0].tokens_in_window, 85_000);
    let limit = result[0].context_limit;
    assert!(limit > 0);
    let pct = result[0].pct_used;
    assert!((pct - (85_000.0 / limit as f64 * 100.0)).abs() < 0.1);
}
