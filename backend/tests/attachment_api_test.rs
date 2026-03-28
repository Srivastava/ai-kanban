use ai_kanban_backend::api::AppState;
use ai_kanban_backend::claude::{ClaudeManager, SessionQueue};
use ai_kanban_backend::db::{
    create_pool, AttachmentRepository, CommentRepository, LogRepository, OtelMetricsRepository,
    SessionMetricsRepository, SessionRepository, SettingsRepository, TaskRepository,
    TokenEventRepository,
};
use axum_test::http::StatusCode;
use axum_test::multipart::MultipartForm;
use axum_test::TestServer;
use std::sync::Arc;

async fn setup_test_server_with_dir(attachments_dir: &str) -> TestServer {
    let db_path = format!("/tmp/test-att-api-{}.db", uuid::Uuid::new_v4());
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
        session_repo.clone(),
        token_event_repo.clone(),
        session_metrics_repo.clone(),
        comment_repo.clone(),
        task_repo.clone(),
        otel_metrics_repo.clone(),
        None,
        None,
        attachment_repo.clone(),
    ));
    let queue = Arc::new(SessionQueue::new(manager, task_repo.clone()));
    let state = AppState::new(
        task_repo,
        log_repo,
        session_repo,
        comment_repo,
        token_event_repo,
        session_metrics_repo,
        settings_repo,
        otel_metrics_repo,
        attachment_repo,
    )
    .with_queue(queue);

    // Set ATTACHMENTS_DIR before creating the router
    std::env::set_var("ATTACHMENTS_DIR", attachments_dir);
    TestServer::new(ai_kanban_backend::api::create_router(state)).unwrap()
}

/// Helper: create a task via POST /api/tasks and return its id
async fn create_task(server: &TestServer) -> String {
    let resp = server
        .post("/api/tasks")
        .json(&serde_json::json!({
            "title": "Test Task",
            "project_path": "/tmp/test"
        }))
        .await;
    assert_eq!(resp.status_code(), StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    body["id"].as_str().unwrap().to_string()
}

/// A minimal 1×1 PNG file (67 bytes)
fn minimal_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // width=1, height=1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth, color type, etc.
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21,
        0xbc, 0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, // IEND chunk
        0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

// ==================== Test 1 ====================

#[tokio::test]
async fn list_attachments_empty_for_new_task() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    let resp = server
        .get(&format!("/api/tasks/{}/attachments", task_id))
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let items: Vec<serde_json::Value> = resp.json();
    assert!(items.is_empty());
}

// ==================== Test 2 ====================

#[tokio::test]
async fn upload_attachment_returns_200_with_json() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_type("image/png"),
    );

    let resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(!body["id"].as_str().unwrap().is_empty());
    assert_eq!(body["task_id"].as_str().unwrap(), task_id);
    assert_eq!(body["filename"].as_str().unwrap(), "test.png");
    assert_eq!(body["mime_type"].as_str().unwrap(), "image/png");
    assert!(body["size_bytes"].as_i64().unwrap() > 0);
}

// ==================== Test 3 ====================

#[tokio::test]
async fn upload_unknown_task_returns_404() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;

    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_type("image/png"),
    );

    let resp = server
        .post("/api/tasks/nonexistent-task-id/attachments")
        .multipart(form)
        .await;

    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
}

// ==================== Test 4 ====================

#[tokio::test]
async fn upload_no_file_field_returns_400() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    // Send an empty multipart form (no parts) — the handler falls through the
    // while loop and returns Err(StatusCode::BAD_REQUEST)
    let form = MultipartForm::new();

    let resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;

    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

// ==================== Test 5 ====================

#[tokio::test]
async fn list_after_upload_returns_one_item() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(minimal_png())
            .file_name("image.png")
            .mime_type("image/png"),
    );

    let upload_resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;
    assert_eq!(upload_resp.status_code(), StatusCode::OK);

    let list_resp = server
        .get(&format!("/api/tasks/{}/attachments", task_id))
        .await;
    assert_eq!(list_resp.status_code(), StatusCode::OK);
    let items: Vec<serde_json::Value> = list_resp.json();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["filename"].as_str().unwrap(), "image.png");
}

// ==================== Test 6 ====================

#[tokio::test]
async fn delete_attachment_returns_204() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    // Upload
    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(minimal_png())
            .file_name("todelete.png")
            .mime_type("image/png"),
    );
    let upload_resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;
    assert_eq!(upload_resp.status_code(), StatusCode::OK);
    let att: serde_json::Value = upload_resp.json();
    let att_id = att["id"].as_str().unwrap();

    // Delete
    let del_resp = server
        .delete(&format!("/api/tasks/{}/attachments/{}", task_id, att_id))
        .await;
    assert_eq!(del_resp.status_code(), StatusCode::NO_CONTENT);

    // List should be empty again
    let list_resp = server
        .get(&format!("/api/tasks/{}/attachments", task_id))
        .await;
    let items: Vec<serde_json::Value> = list_resp.json();
    assert!(items.is_empty());
}

// ==================== Test 7 ====================

#[tokio::test]
async fn delete_wrong_task_returns_404() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    // Upload attachment under the real task
    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(minimal_png())
            .file_name("file.png")
            .mime_type("image/png"),
    );
    let upload_resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;
    assert_eq!(upload_resp.status_code(), StatusCode::OK);
    let att: serde_json::Value = upload_resp.json();
    let att_id = att["id"].as_str().unwrap();

    // Try to delete using a wrong task_id
    let del_resp = server
        .delete(&format!("/api/tasks/wrong-task-id/attachments/{}", att_id))
        .await;
    assert_eq!(del_resp.status_code(), StatusCode::NOT_FOUND);
}

// ==================== Test 8 ====================

#[tokio::test]
async fn serve_file_bytes_with_correct_content_type() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;
    let file_bytes = minimal_png();

    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(file_bytes.clone())
            .file_name("picture.png")
            .mime_type("image/png"),
    );
    let upload_resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;
    assert_eq!(upload_resp.status_code(), StatusCode::OK);
    let att: serde_json::Value = upload_resp.json();
    let att_id = att["id"].as_str().unwrap();

    let serve_resp = server
        .get(&format!(
            "/api/tasks/{}/attachments/{}/file",
            task_id, att_id
        ))
        .await;

    assert_eq!(serve_resp.status_code(), StatusCode::OK);

    // Check Content-Type header
    let content_type = serve_resp
        .headers()
        .get("content-type")
        .expect("Content-Type header missing")
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("image/png"),
        "Expected image/png, got: {}",
        content_type
    );

    // Check bytes match
    let returned_bytes = serve_resp.as_bytes().to_vec();
    assert_eq!(returned_bytes, file_bytes);
}

// ==================== Test 9 ====================

#[tokio::test]
async fn filename_sanitization_path_traversal() {
    let dir = format!("/tmp/test-att-{}", uuid::Uuid::new_v4());
    std::fs::create_dir_all(&dir).unwrap();
    let server = setup_test_server_with_dir(&dir).await;
    let task_id = create_task(&server).await;

    let form = MultipartForm::new().add_part(
        "file",
        axum_test::multipart::Part::bytes(minimal_png())
            .file_name("../../../etc/passwd")
            .mime_type("image/png"),
    );

    let resp = server
        .post(&format!("/api/tasks/{}/attachments", task_id))
        .multipart(form)
        .await;

    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();

    // The storage_path must not contain path traversal
    let storage_path = body["storage_path"].as_str().unwrap();
    assert!(
        !storage_path.contains("../"),
        "storage_path contains path traversal: {}",
        storage_path
    );
    assert!(
        !storage_path.contains("/etc/passwd"),
        "storage_path contains /etc/passwd: {}",
        storage_path
    );
}
