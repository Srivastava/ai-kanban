use ai_kanban_backend::api::{create_router, AppState};
use ai_kanban_backend::claude::ClaudeManager;
use ai_kanban_backend::db::{create_pool, CommentRepository, LogRepository, SessionMetricsRepository, SessionRepository, TaskRepository, TokenEventRepository};
use ai_kanban_backend::logging::DbLayer;
use axum::Extension;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/ai-kanban.db".into());
    let pool = create_pool(&db_path).await?;

    // Create repositories
    let task_repo = TaskRepository::new(pool.clone());
    let log_repo = LogRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let token_event_repo = TokenEventRepository::new(pool.clone());
    let session_metrics_repo = SessionMetricsRepository::new(pool.clone());

    // Initialize Claude manager and session queue
    let claude_manager = Arc::new(ClaudeManager::new(
        session_repo.clone(),
        token_event_repo.clone(),
        session_metrics_repo.clone(),
    ));
    let queue = Arc::new(ai_kanban_backend::claude::SessionQueue::new(
        claude_manager.clone(),
        task_repo.clone(),
    ));

    // Initialize logging with DB layer
    let db_layer = DbLayer::new(log_repo.clone());
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(db_layer)
        .init();

    tracing::info!("Database initialized at {}", db_path);
    tracing::info!("Logging system initialized");

    // Create state with queue
    let state = AppState::new(
        task_repo,
        log_repo,
        session_repo,
        comment_repo,
        token_event_repo,
        session_metrics_repo,
    ).with_queue(queue);
    tracing::debug!("Application state created");

    // Build app with CORS and Extension for WebSocket
    let app = create_router(state)
        .layer(Extension(claude_manager.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    tracing::info!("Server shutdown");
    Ok(())
}
