use crate::models::{CreateLog, Log, LogFilter};
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct LogRepository {
    pool: SqlitePool,
}

impl LogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, create: CreateLog) -> Result<Log> {
        let now = chrono::Utc::now();
        let source = create.source.unwrap_or_else(|| "backend".to_string());
        let metadata = create.metadata.map(|m| m.to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO logs (timestamp, level, message, target, source, task_id, session_id, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(&create.level)
        .bind(&create.message)
        .bind(&create.target)
        .bind(&source)
        .bind(&create.task_id)
        .bind(&create.session_id)
        .bind(&metadata)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Log {
            id: result.last_insert_rowid(),
            timestamp: now,
            level: create.level,
            message: create.message,
            target: create.target,
            source,
            task_id: create.task_id,
            session_id: create.session_id,
            metadata,
            created_at: now,
        })
    }

    pub async fn list(&self, filter: LogFilter) -> Result<Vec<Log>> {
        let limit = filter.limit.unwrap_or(100).min(1000);
        let offset = filter.offset.unwrap_or(0);

        let mut query = String::from(
            "SELECT * FROM logs WHERE 1=1"
        );
        let mut binds: Vec<String> = Vec::new();

        if let Some(level) = &filter.level {
            query.push_str(&format!(" AND level = ?"));
            binds.push(level.clone());
        }
        if let Some(source) = &filter.source {
            query.push_str(&format!(" AND source = ?"));
            binds.push(source.clone());
        }
        if let Some(task_id) = &filter.task_id {
            query.push_str(&format!(" AND task_id = ?"));
            binds.push(task_id.clone());
        }
        if let Some(session_id) = &filter.session_id {
            query.push_str(&format!(" AND session_id = ?"));
            binds.push(session_id.clone());
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut sql_query = sqlx::query_as::<_, Log>(&query);
        for bind in binds {
            sql_query = sql_query.bind(bind);
        }
        sql_query = sql_query.bind(limit).bind(offset);

        let logs = sql_query.fetch_all(&self.pool).await?;
        Ok(logs)
    }

    pub async fn list_by_task(&self, task_id: &str, limit: Option<i32>) -> Result<Vec<Log>> {
        let limit = limit.unwrap_or(100).min(1000);
        let logs = sqlx::query_as::<_, Log>(
            "SELECT * FROM logs WHERE task_id = ? ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(task_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }

    pub async fn delete_old_logs(&self, days: i32) -> Result<u64> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let result = sqlx::query("DELETE FROM logs WHERE timestamp < ?")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
