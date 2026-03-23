use ai_kanban_backend::api::AppState;
use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
use ai_kanban_backend::db::{AttachmentRepository, create_pool, CommentRepository, LogRepository, OtelMetricsRepository, SessionMetricsRepository, SessionRepository, SettingsRepository, TaskRepository, TokenEventRepository};
use axum_test::TestServer;
use axum_test::http::StatusCode;
use std::sync::Arc;

async fn setup_test_server() -> TestServer {
    let db_path = format!("/tmp/test-api-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    let task_repo = TaskRepository::new(pool.clone());
    let log_repo = LogRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let token_event_repo = TokenEventRepository::new(pool.clone());
    let session_metrics_repo = SessionMetricsRepository::new(pool.clone());
    let settings_repo = SettingsRepository::new(pool.clone());
    let otel_metrics_repo = OtelMetricsRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());
    let manager = Arc::new(ClaudeManager::new(
        session_repo.clone(), token_event_repo.clone(), session_metrics_repo.clone(),
        comment_repo.clone(), task_repo.clone(), otel_metrics_repo.clone(), None, None,
        attachment_repo.clone(),
    ));
    let queue = Arc::new(SessionQueue::new(manager, task_repo.clone()));
    let state = AppState::new(task_repo, log_repo, session_repo, comment_repo, token_event_repo, session_metrics_repo, settings_repo, otel_metrics_repo, attachment_repo).with_queue(queue);
    TestServer::new(ai_kanban_backend::api::create_router(state)).unwrap()
}

// ==================== Health Check ====================

#[tokio::test]
async fn test_health_endpoint() {
    let server = setup_test_server().await;

    let response = server.get("/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text(), "ok");
}

// ==================== Task API Tests ====================

#[tokio::test]
async fn test_api_create_task() {
    let server = setup_test_server().await;

    let response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Test Task",
            "description": "Test description",
            "project_path": "/tmp/test"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let task: serde_json::Value = response.json();
    assert!(!task["id"].as_str().unwrap().is_empty());
    assert_eq!(task["title"], "Test Task");
    assert_eq!(task["stage"], "backlog");
}

#[tokio::test]
async fn test_api_create_task_minimal() {
    let server = setup_test_server().await;

    let response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Minimal Task",
            "project_path": "/tmp/test"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let task: serde_json::Value = response.json();
    assert_eq!(task["description"], serde_json::Value::Null);
}

#[tokio::test]
async fn test_api_list_tasks_empty() {
    let server = setup_test_server().await;

    let response = server.get("/api/tasks").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let tasks: Vec<serde_json::Value> = response.json();
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn test_api_list_tasks_with_data() {
    let server = setup_test_server().await;

    // Create two tasks
    server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task 1",
            "project_path": "/tmp/test"
        }))
        .await;

    server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task 2",
            "project_path": "/tmp/test"
        }))
        .await;

    let response = server.get("/api/tasks").await;
    let tasks: Vec<serde_json::Value> = response.json();

    assert_eq!(tasks.len(), 2);
}

#[tokio::test]
async fn test_api_list_tasks_filter_by_stage() {
    let server = setup_test_server().await;

    // Create task and move to in_progress
    let create_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = create_response.json();
    let task_id = task["id"].as_str().unwrap();

    server
        .post(&format!("/api/tasks/{}/move", task_id))
        .json(&serde_json::json!({ "stage": "in_progress" }))
        .await;

    // Filter by backlog (should be empty)
    let backlog_response = server
        .get("/api/tasks")
        .add_query_params(&[("stage", "backlog")])
        .await;
    let backlog_tasks: Vec<serde_json::Value> = backlog_response.json();
    assert_eq!(backlog_tasks.len(), 0);

    // Filter by in_progress (should have 1)
    let in_progress_response = server
        .get("/api/tasks")
        .add_query_params(&[("stage", "in_progress")])
        .await;
    let in_progress_tasks: Vec<serde_json::Value> = in_progress_response.json();
    assert_eq!(in_progress_tasks.len(), 1);
}

#[tokio::test]
async fn test_api_get_task() {
    let server = setup_test_server().await;

    let create_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Test Task",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = create_response.json();
    let task_id = task["id"].as_str().unwrap();

    let response = server.get(&format!("/api/tasks/{}", task_id)).await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let fetched: serde_json::Value = response.json();
    assert_eq!(fetched["id"], task_id);
    assert_eq!(fetched["title"], "Test Task");
}

#[tokio::test]
async fn test_api_get_task_not_found() {
    let server = setup_test_server().await;

    let response = server.get("/api/tasks/nonexistent-id").await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    let error: serde_json::Value = response.json();
    assert!(error["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_api_update_task() {
    let server = setup_test_server().await;

    let create_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Original Title",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = create_response.json();
    let task_id = task["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/tasks/{}", task_id))
        .json(&serde_json::json!({
            "title": "Updated Title",
            "description": "New description"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let updated: serde_json::Value = response.json();
    assert_eq!(updated["title"], "Updated Title");
    assert_eq!(updated["description"], "New description");
}

#[tokio::test]
async fn test_api_update_task_not_found() {
    let server = setup_test_server().await;

    let response = server
        .patch("/api/tasks/nonexistent-id")
        .json(&serde_json::json!({ "title": "New Title" }))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_delete_task() {
    let server = setup_test_server().await;

    let create_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "To Delete",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = create_response.json();
    let task_id = task["id"].as_str().unwrap();

    let delete_response = server.delete(&format!("/api/tasks/{}", task_id)).await;
    assert_eq!(delete_response.status_code(), StatusCode::NO_CONTENT);

    // Verify deleted
    let get_response = server.get(&format!("/api/tasks/{}", task_id)).await;
    assert_eq!(get_response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_delete_task_not_found() {
    let server = setup_test_server().await;

    let response = server.delete("/api/tasks/nonexistent-id").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_move_task() {
    let server = setup_test_server().await;

    let create_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task to Move",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = create_response.json();
    let task_id = task["id"].as_str().unwrap();

    let response = server
        .post(&format!("/api/tasks/{}/move", task_id))
        .json(&serde_json::json!({ "stage": "in_progress" }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let moved: serde_json::Value = response.json();
    assert_eq!(moved["stage"], "in_progress");
}

#[tokio::test]
async fn test_api_move_task_all_stages() {
    let server = setup_test_server().await;

    let create_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = create_response.json();
    let task_id = task["id"].as_str().unwrap();

    let stages = ["planning", "ready", "in_progress", "review", "done"];

    for stage in stages {
        let response = server
            .post(&format!("/api/tasks/{}/move", task_id))
            .json(&serde_json::json!({ "stage": stage }))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let moved: serde_json::Value = response.json();
        assert_eq!(moved["stage"], stage);
    }
}

// ==================== Log API Tests ====================

#[tokio::test]
async fn test_api_create_log() {
    let server = setup_test_server().await;

    let response = server
        .post("/api/logs")
        .json(&serde_json::json!({
            "level": "INFO",
            "message": "Test log message",
            "source": "frontend"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let log: serde_json::Value = response.json();
    assert!(log["id"].as_i64().unwrap() > 0);
    assert_eq!(log["level"], "INFO");
    assert_eq!(log["message"], "Test log message");
    assert_eq!(log["source"], "frontend");
}

#[tokio::test]
async fn test_api_create_log_default_source() {
    let server = setup_test_server().await;

    let response = server
        .post("/api/logs")
        .json(&serde_json::json!({
            "level": "INFO",
            "message": "Test"
        }))
        .await;

    let log: serde_json::Value = response.json();
    assert_eq!(log["source"], "frontend");
}

#[tokio::test]
async fn test_api_create_log_invalid_level() {
    let server = setup_test_server().await;

    let response = server
        .post("/api/logs")
        .json(&serde_json::json!({
            "level": "INVALID",
            "message": "Test"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_api_create_log_all_levels() {
    let server = setup_test_server().await;

    for level in ["DEBUG", "INFO", "WARN", "ERROR"] {
        let response = server
            .post("/api/logs")
            .json(&serde_json::json!({
                "level": level,
                "message": format!("{} message", level)
            }))
            .await;

        assert_eq!(response.status_code(), StatusCode::CREATED);

        let log: serde_json::Value = response.json();
        assert_eq!(log["level"], level);
    }
}

#[tokio::test]
async fn test_api_create_log_with_context() {
    let server = setup_test_server().await;

    // Create a task first
    let task_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;

    let task: serde_json::Value = task_response.json();
    let task_id = task["id"].as_str().unwrap();

    let response = server
        .post("/api/logs")
        .json(&serde_json::json!({
            "level": "INFO",
            "message": "Task related log",
            "task_id": task_id,
            "session_id": "session-123",
            "metadata": {"key": "value"}
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let log: serde_json::Value = response.json();
    assert_eq!(log["task_id"], task_id);
    assert_eq!(log["session_id"], "session-123");
}

#[tokio::test]
async fn test_api_list_logs() {
    let server = setup_test_server().await;

    // Create some logs
    for i in 0..3 {
        server
            .post("/api/logs")
            .json(&serde_json::json!({
                "level": "INFO",
                "message": format!("Log {}", i)
            }))
            .await;
    }

    let response = server.get("/api/logs").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let logs: Vec<serde_json::Value> = response.json();
    assert!(logs.len() >= 3); // May include backend logs too
}

#[tokio::test]
async fn test_api_list_logs_filter_by_level() {
    let server = setup_test_server().await;

    server
        .post("/api/logs")
        .json(&serde_json::json!({ "level": "ERROR", "message": "Error" }))
        .await;

    server
        .post("/api/logs")
        .json(&serde_json::json!({ "level": "INFO", "message": "Info" }))
        .await;

    let response = server
        .get("/api/logs")
        .add_query_params(&[("level", "ERROR")])
        .await;
    let logs: Vec<serde_json::Value> = response.json();

    assert!(logs.iter().all(|l| l["level"] == "ERROR"));
}

#[tokio::test]
async fn test_api_list_logs_filter_by_source() {
    let server = setup_test_server().await;

    server
        .post("/api/logs")
        .json(&serde_json::json!({
            "level": "INFO",
            "message": "Frontend log",
            "source": "frontend"
        }))
        .await;

    let response = server
        .get("/api/logs")
        .add_query_params(&[("source", "frontend")])
        .await;
    let logs: Vec<serde_json::Value> = response.json();

    assert!(logs.iter().all(|l| l["source"] == "frontend"));
}

#[tokio::test]
async fn test_api_list_logs_pagination() {
    let server = setup_test_server().await;

    // Create 20 logs
    for i in 0..20 {
        server
            .post("/api/logs")
            .json(&serde_json::json!({
                "level": "INFO",
                "message": format!("Log {}", i),
                "source": "frontend"
            }))
            .await;
    }

    let page1 = server
        .get("/api/logs")
        .add_query_params(&[("limit", "5"), ("offset", "0"), ("source", "frontend")])
        .await;
    let page2 = server
        .get("/api/logs")
        .add_query_params(&[("limit", "5"), ("offset", "5"), ("source", "frontend")])
        .await;

    let logs1: Vec<serde_json::Value> = page1.json();
    let logs2: Vec<serde_json::Value> = page2.json();

    assert_eq!(logs1.len(), 5);
    assert_eq!(logs2.len(), 5);

    // Pages should have different IDs
    let ids1: Vec<_> = logs1.iter().map(|l| l["id"].as_i64()).collect();
    let ids2: Vec<_> = logs2.iter().map(|l| l["id"].as_i64()).collect();
    assert!(!ids1.iter().any(|id| ids2.contains(id)));
}

// ==================== Comment API Tests ====================

#[tokio::test]
async fn test_api_list_comments_empty() {
    let server = setup_test_server().await;

    // Create a task first
    let task_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;
    let task: serde_json::Value = task_response.json();
    let task_id = task["id"].as_str().unwrap();

    let response = server
        .get(&format!("/api/tasks/{}/comments", task_id))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let comments: Vec<serde_json::Value> = response.json();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_api_create_comment() {
    let server = setup_test_server().await;

    // Create a task first
    let task_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;
    let task: serde_json::Value = task_response.json();
    let task_id = task["id"].as_str().unwrap();

    let response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&serde_json::json!({
            "content": "Test comment"
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);

    let comment: serde_json::Value = response.json();
    assert!(!comment["id"].as_str().unwrap().is_empty());
    assert_eq!(comment["task_id"], task_id);
    assert_eq!(comment["content"], "Test comment");
    assert_eq!(comment["author"], "user");
    assert!(comment["parent_id"].is_null());
}

#[tokio::test]
async fn test_api_create_comment_with_parent() {
    let server = setup_test_server().await;

    // Create a task
    let task_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;
    let task: serde_json::Value = task_response.json();
    let task_id = task["id"].as_str().unwrap();

    // Create parent comment
    let parent_response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&serde_json::json!({
            "content": "Parent comment"
        }))
        .await;
    let parent: serde_json::Value = parent_response.json();
    let parent_id = parent["id"].as_str().unwrap();

    // Create reply
    let reply_response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&serde_json::json!({
            "content": "Reply comment",
            "parent_id": parent_id
        }))
        .await;

    assert_eq!(reply_response.status_code(), StatusCode::CREATED);

    let reply: serde_json::Value = reply_response.json();
    assert_eq!(reply["parent_id"], parent_id);
}

#[tokio::test]
async fn test_api_list_comments_with_replies() {
    let server = setup_test_server().await;

    // Create a task
    let task_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;
    let task: serde_json::Value = task_response.json();
    let task_id = task["id"].as_str().unwrap();

    // Create parent comment
    let parent_response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&serde_json::json!({
            "content": "Parent"
        }))
        .await;
    let parent: serde_json::Value = parent_response.json();
    let parent_id = parent["id"].as_str().unwrap();

    // Create reply
    server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&serde_json::json!({
            "content": "Reply",
            "parent_id": parent_id
        }))
        .await;

    // List comments
    let response = server
        .get(&format!("/api/tasks/{}/comments", task_id))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let comments: Vec<serde_json::Value> = response.json();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["id"], parent_id);
    assert_eq!(comments[0]["replies"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_api_delete_comment() {
    let server = setup_test_server().await;

    // Create a task
    let task_response = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Task",
            "project_path": "/tmp/test"
        }))
        .await;
    let task: serde_json::Value = task_response.json();
    let task_id = task["id"].as_str().unwrap();

    // Create comment
    let comment_response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&serde_json::json!({
            "content": "To be deleted"
        }))
        .await;
    let comment: serde_json::Value = comment_response.json();
    let comment_id = comment["id"].as_str().unwrap();

    // Delete the comment
    let delete_response = server.delete(&format!("/api/comments/{}", comment_id)).await;
    assert_eq!(delete_response.status_code(), StatusCode::NO_CONTENT);

    // Verify it's gone
    let list_response = server
        .get(&format!("/api/tasks/{}/comments", task_id))
        .await;
    let comments: Vec<serde_json::Value> = list_response.json();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_api_delete_comment_not_found() {
    let server = setup_test_server().await;

    let response = server.delete("/api/comments/nonexistent-id").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

// ==================== New Analytics API Tests ====================

#[tokio::test]
async fn test_analytics_cost_by_task_empty() {
    let server = setup_test_server().await;
    let response = server.get("/api/analytics/cost/by-task").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: Vec<serde_json::Value> = response.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_analytics_tokens_by_stage_empty() {
    let server = setup_test_server().await;
    let response = server.get("/api/analytics/tokens/by-stage").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: Vec<serde_json::Value> = response.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_analytics_session_summary_zero() {
    let server = setup_test_server().await;
    let response = server.get("/api/analytics/sessions/summary").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["total_sessions"], 0);
}

#[tokio::test]
async fn test_analytics_burn_rate_structure() {
    let server = setup_test_server().await;
    let response = server.get("/api/analytics/burn-rate").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(body["tokens_last_hour"].is_number());
    assert!(body["tokens_per_minute"].is_number());
}

// ==================== Sessions API Tests ====================

#[tokio::test]
async fn test_api_list_sessions_empty() {
    let server = setup_test_server().await;
    let response = server.get("/api/sessions").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["active_count"], 0);
    assert!(body["queued"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_get_session_not_found() {
    let server = setup_test_server().await;
    let response = server.get("/api/sessions/nonexistent-id").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_stop_session_not_found() {
    let server = setup_test_server().await;
    let response = server.post("/api/sessions/nonexistent-id/stop").await;
    // stop_session returns 200 OK even for nonexistent IDs: the implementation
    // silently no-ops (removes nothing from active_sessions) and returns Ok(()),
    // so the handler emits a 200 with { "status": "stopped" }.
    assert_eq!(response.status_code(), StatusCode::OK);
}

// ==================== Task API Error Path Tests ====================

#[tokio::test]
async fn test_api_start_session_task_not_found() {
    let server = setup_test_server().await;
    let response = server.post("/api/tasks/nonexistent-id/sessions").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_continue_session_task_not_found() {
    let server = setup_test_server().await;
    let response = server.post("/api/tasks/nonexistent-id/sessions/continue").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_update_task_stage() {
    let server = setup_test_server().await;

    let create_resp = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Stage Test",
            "project_path": "/tmp"
        }))
        .await;
    let task: serde_json::Value = create_resp.json();
    let task_id = task["id"].as_str().unwrap();

    let update_resp = server
        .patch(&format!("/api/tasks/{}", task_id))
        .json(&serde_json::json!({ "stage": "ready" }))
        .await;
    assert_eq!(update_resp.status_code(), StatusCode::OK);
    let updated: serde_json::Value = update_resp.json();
    assert_eq!(updated["stage"], "ready");
}

// ==================== Analytics API Endpoint Tests ====================

#[tokio::test]
async fn test_api_analytics_overview() {
    let server = setup_test_server().await;
    let response = server.get("/api/analytics/overview").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_daily_tokens() {
    let server = setup_test_server().await;
    // Route is /api/analytics/tokens/daily
    let response = server.get("/api/analytics/tokens/daily").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_tokens_by_task() {
    let server = setup_test_server().await;
    // Route is /api/analytics/tokens/by-task
    let response = server.get("/api/analytics/tokens/by-task").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_tokens_by_session() {
    let server = setup_test_server().await;
    // Route is /api/analytics/tokens/by-session
    let response = server.get("/api/analytics/tokens/by-session").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_tokens_by_tool() {
    let server = setup_test_server().await;
    // Route is /api/analytics/tokens/by-tool
    let response = server.get("/api/analytics/tokens/by-tool").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_tokens_by_language() {
    let server = setup_test_server().await;
    // Route is /api/analytics/tokens/by-language
    let response = server.get("/api/analytics/tokens/by-language").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_token_efficiency() {
    let server = setup_test_server().await;
    // Route is /api/analytics/tokens/efficiency
    let response = server.get("/api/analytics/tokens/efficiency").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_analytics_usage_windows() {
    let server = setup_test_server().await;
    let response = server.get("/api/analytics/usage-windows").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

// ==================== Analytics New Endpoints ====================

#[tokio::test]
async fn test_analytics_plan_tier_endpoint() {
    // Clear env vars so we get a deterministic "pro" default
    std::env::remove_var("CLAUDE_PLAN_TIER");
    std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
    std::env::remove_var("CLAUDE_WEEKLY_TOKEN_LIMIT");

    let server = setup_test_server().await;
    let resp = server.get("/api/analytics/plan-tier").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["tier"].is_string());
    assert!(body["limit_5hr"].as_i64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn test_analytics_roi_endpoint() {
    let server = setup_test_server().await;
    let resp = server.get("/api/analytics/roi").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["total_commits"].is_number());
    assert!(body["total_cost_usd"].is_number());
}

#[tokio::test]
async fn test_analytics_context_usage_endpoint() {
    let server = setup_test_server().await;
    let resp = server.get("/api/analytics/context-usage").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body.is_array(), "context-usage should return an array");
}
