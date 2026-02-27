use ai_kanban_backend::db::create_pool;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize database
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/ai-kanban.db".into());
    let _pool = create_pool(&db_path).await?;
    tracing::info!("Database initialized at {}", db_path);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/db-health", get(|| async { "db ok" }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
