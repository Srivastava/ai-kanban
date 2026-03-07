use crate::models::{CreateOtelLog, OtelLog};
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct OtelLogsRepository {
    pool: SqlitePool,
}

impl OtelLogsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, log: CreateOtelLog) -> Result<OtelLog> {
        let attrs = serde_json::to_string(&log.attributes)?;
        let row = sqlx::query_as!(
            OtelLog,
            r#"INSERT INTO otel_logs
               (event_name, body, severity_text, severity_number,
                session_id, task_id, claude_session_id, attributes, otel_timestamp)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, event_name, body, severity_text, severity_number,
                         session_id, task_id, claude_session_id, attributes, otel_timestamp,
                         created_at as "created_at: _""#,
            log.event_name,
            log.body,
            log.severity_text,
            log.severity_number,
            log.session_id,
            log.task_id,
            log.claude_session_id,
            attrs,
            log.otel_timestamp,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn correlate(&self, claude_session_id: &str, session_id: &str, task_id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE otel_logs SET session_id = ?, task_id = ?
             WHERE claude_session_id = ? AND session_id IS NULL",
            session_id, task_id, claude_session_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
