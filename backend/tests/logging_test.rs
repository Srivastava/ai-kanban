//! Tests for the logging infrastructure
//! 
//! Note: The DbLayer tracing subscriber runs in a background thread,
//! making it difficult to test directly. These tests focus on the
//! LogRepository and model functionality which is what matters for
//! the logging system.

use ai_kanban_backend::db::{create_pool, LogRepository};
use ai_kanban_backend::models::{CreateLog, LogFilter};

async fn setup_log_repo() -> LogRepository {
    let db_path = format!("/tmp/test-logging-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    LogRepository::new(pool)
}

// The logging system is tested indirectly through the log repository
// and API tests. The DbLayer subscriber is infrastructure that writes
// to the same repository.

#[tokio::test]
async fn test_logging_infrastructure_basic() {
    let repo = setup_log_repo().await;
    
    // Simulate what the tracing layer does - write a log
    let log = repo.create(CreateLog {
        level: "INFO".to_string(),
        message: "Backend tracing event".to_string(),
        target: Some("module::path".to_string()),
        source: Some("backend".to_string()),
        task_id: None,
        session_id: None,
        metadata: Some(serde_json::json!({ "span": "test" })),
    }).await.expect("Failed to create log");
    
    assert!(log.id > 0);
    assert_eq!(log.source, "backend");
}

#[tokio::test]
async fn test_logging_frontend_source() {
    let repo = setup_log_repo().await;
    
    let log = repo.create(CreateLog {
        level: "INFO".to_string(),
        message: "Frontend event".to_string(),
        target: None,
        source: Some("frontend".to_string()),
        task_id: None,
        session_id: None,
        metadata: None,
    }).await.expect("Failed to create log");
    
    assert_eq!(log.source, "frontend");
}

#[tokio::test]
async fn test_logging_mixed_sources() {
    let repo = setup_log_repo().await;
    
    // Create logs from different sources
    for source in ["backend", "frontend", "backend", "frontend"] {
        repo.create(CreateLog {
            level: "INFO".to_string(),
            message: format!("{} log", source),
            target: None,
            source: Some(source.to_string()),
            task_id: None,
            session_id: None,
            metadata: None,
        }).await.unwrap();
    }
    
    let backend_count = repo.list(LogFilter {
        source: Some("backend".to_string()),
        ..Default::default()
    }).await.unwrap().len();
    
    let frontend_count = repo.list(LogFilter {
        source: Some("frontend".to_string()),
        ..Default::default()
    }).await.unwrap().len();
    
    assert_eq!(backend_count, 2);
    assert_eq!(frontend_count, 2);
}
