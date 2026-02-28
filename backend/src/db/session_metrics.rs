use crate::models::SessionMetrics;
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SessionMetricsRepository {
    pool: SqlitePool,
}

impl SessionMetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, session_id: &str, project_files: i64, project_loc: i64) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            INSERT INTO session_metrics (session_id, project_files, project_loc, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(session_id) DO UPDATE SET
                project_files = excluded.project_files,
                project_loc   = excluded.project_loc,
                updated_at    = excluded.updated_at
            "#,
        )
        .bind(session_id)
        .bind(project_files)
        .bind(project_loc)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find(&self, session_id: &str) -> Result<Option<SessionMetrics>> {
        let metrics = sqlx::query_as::<_, SessionMetrics>(
            "SELECT * FROM session_metrics WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(metrics)
    }

    pub async fn add_lines_written(&self, session_id: &str, lines: i64) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE session_metrics SET lines_written = lines_written + ?, updated_at = ? WHERE session_id = ?",
        )
        .bind(lines)
        .bind(now.to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_lines_deleted(&self, session_id: &str, lines: i64) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE session_metrics SET lines_deleted = lines_deleted + ?, updated_at = ? WHERE session_id = ?",
        )
        .bind(lines)
        .bind(now.to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
