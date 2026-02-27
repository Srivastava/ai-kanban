use sqlx::SqlitePool;
use std::path::Path;

pub async fn create_pool(db_path: &str) -> anyhow::Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
