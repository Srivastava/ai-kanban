use sqlx::pool::PoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

pub async fn create_pool(db_path: &str) -> anyhow::Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(30))
        .foreign_keys(true);

    // WAL mode allows multiple concurrent readers with one writer.
    // 5 connections prevents pool starvation when the logging layer and API
    // handlers compete for connections simultaneously.
    // Exception: `:memory:` creates a separate isolated DB per connection,
    // so tests that use in-memory DBs must stay at 1 connection.
    let max_conn = if db_path == ":memory:" { 1 } else { 5 };
    let pool = PoolOptions::new()
        .max_connections(max_conn)
        .connect_with(options)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
