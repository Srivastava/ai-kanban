use ai_kanban_backend::db::{create_pool, LogRepository};
use ai_kanban_backend::logging::DbLayer;
use ai_kanban_backend::models::LogFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

async fn setup() -> LogRepository {
    let db_path = format!("/tmp/test-logging-layer-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    LogRepository::new(pool)
}

/// Wait for the background thread to flush (polls at 100ms intervals, so 400ms is safe)
async fn flush() {
    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
}

#[tokio::test]
async fn test_db_layer_captures_info_event() {
    let repo = setup().await;
    let layer = DbLayer::new(repo.clone());
    let subscriber = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);
    tracing::info!("unique-info-msg-capture-test");
    drop(_guard);

    flush().await;

    let logs = repo.list(LogFilter::default()).await.unwrap();
    let found = logs
        .iter()
        .any(|l| l.message.contains("unique-info-msg-capture-test"));
    assert!(found, "INFO event should be captured in DB");
}

#[tokio::test]
async fn test_db_layer_captures_warn_event() {
    let repo = setup().await;
    let layer = DbLayer::new(repo.clone());
    let subscriber = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);
    tracing::warn!("unique-warn-msg-capture-test");
    drop(_guard);

    flush().await;

    let logs = repo.list(LogFilter::default()).await.unwrap();
    let found = logs
        .iter()
        .find(|l| l.message.contains("unique-warn-msg-capture-test"));
    assert!(found.is_some(), "WARN event should be captured");
    assert_eq!(found.unwrap().level, "WARN");
}

#[tokio::test]
async fn test_db_layer_captures_error_event() {
    let repo = setup().await;
    let layer = DbLayer::new(repo.clone());
    let subscriber = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);
    tracing::error!("unique-error-msg-capture-test");
    drop(_guard);

    flush().await;

    let logs = repo.list(LogFilter::default()).await.unwrap();
    let found = logs
        .iter()
        .find(|l| l.message.contains("unique-error-msg-capture-test"));
    assert!(found.is_some(), "ERROR event should be captured");
    assert_eq!(found.unwrap().level, "ERROR");
}

#[tokio::test]
async fn test_db_layer_captures_task_id_field() {
    let repo = setup().await;
    let layer = DbLayer::new(repo.clone());
    let subscriber = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);
    tracing::info!(task_id = "task-field-xyz", "msg with task_id field");
    drop(_guard);

    flush().await;

    let logs = repo
        .list(LogFilter {
            task_id: Some("task-field-xyz".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        !logs.is_empty(),
        "Log with task_id field should be stored with that task_id"
    );
}

#[tokio::test]
async fn test_db_layer_captures_session_id_field() {
    let repo = setup().await;
    let layer = DbLayer::new(repo.clone());
    let subscriber = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);
    tracing::info!(session_id = "sess-field-abc", "msg with session_id field");
    drop(_guard);

    flush().await;

    let logs = repo
        .list(LogFilter {
            session_id: Some("sess-field-abc".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        !logs.is_empty(),
        "Log with session_id field should be stored with that session_id"
    );
}

#[tokio::test]
async fn test_db_layer_source_is_backend() {
    let repo = setup().await;
    let layer = DbLayer::new(repo.clone());
    let subscriber = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(subscriber);
    tracing::info!("source-check-msg-backend");
    drop(_guard);

    flush().await;

    let logs = repo.list(LogFilter::default()).await.unwrap();
    let found = logs
        .iter()
        .find(|l| l.message.contains("source-check-msg-backend"));
    assert!(found.is_some());
    assert_eq!(found.unwrap().source, "backend");
}
