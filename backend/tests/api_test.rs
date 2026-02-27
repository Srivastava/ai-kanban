use ai_kanban_backend::api::AppState;
use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
use ai_kanban_backend::db::{create_pool, LogRepository, SessionRepository, TaskRepository};
use axum_test::TestServer;
use axum_test::http::StatusCode;
use std::sync::Arc;

async fn setup_test_server() -> TestServer {
    let db_path = format!("/tmp/test-api-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    let task_repo = TaskRepository::new(pool.clone());
    let log_repo = LogRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);
    let manager = Arc::new(ClaudeManager::new(session_repo.clone()));
    let queue = Arc::new(SessionQueue::new(manager, task_repo.clone()));
    let state = AppState::new(task_repo, log_repo, session_repo).with_queue(queue);
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
