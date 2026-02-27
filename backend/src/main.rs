use ai_kanban_backend::api::{create_router, AppState};
use ai_kanban_backend::db::{create_pool, LogRepository, TaskRepository};
use ai_kanban_backend::logging::DbLayer;
use std::net::SocketAddr;
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

    // Create state
    let state = AppState::new(task_repo, log_repo);
    tracing::debug!("Application state created");

    // Build app with CORS
    let app = create_router(state).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    tracing::info!("Server shutdown");
    Ok(())
}
