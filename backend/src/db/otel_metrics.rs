use crate::models::{CreateOtelMetric, DevActivityRow, OtelMetric};
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct OtelMetricsRepository {
    pool: SqlitePool,
}

impl OtelMetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, m: CreateOtelMetric) -> Result<OtelMetric> {
        let attrs = serde_json::to_string(&m.attributes)?;
        let row = sqlx::query_as!(
            OtelMetric,
            r#"INSERT INTO otel_metrics
               (metric_name, value, unit, session_id, task_id, claude_session_id, attributes, otel_timestamp)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, metric_name, value, unit, session_id, task_id,
                         claude_session_id, attributes, otel_timestamp,
                         created_at as "created_at: _""#,
            m.metric_name, m.value, m.unit, m.session_id, m.task_id,
            m.claude_session_id, attrs, m.otel_timestamp
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn correlate(&self, claude_session_id: &str, session_id: &str, task_id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE otel_metrics SET session_id = ?, task_id = ?
             WHERE claude_session_id = ? AND session_id IS NULL",
            session_id, task_id, claude_session_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn dev_activity(&self) -> Result<Vec<DevActivityRow>> {
        let rows = sqlx::query_as!(
            DevActivityRow,
            r#"SELECT
                 om.task_id    as "task_id!",
                 t.title       as "task_title!",
                 om.session_id as "session_id!",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.lines_of_code.count'
                                    AND json_extract(om.attributes, '$.type') = 'added'
                                   THEN om.value ELSE 0 END), 0) as "lines_added!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.lines_of_code.count'
                                    AND json_extract(om.attributes, '$.type') = 'removed'
                                   THEN om.value ELSE 0 END), 0) as "lines_deleted!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.commit.count'
                                   THEN om.value ELSE 0 END), 0) as "commits!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.pull_request.count'
                                   THEN om.value ELSE 0 END), 0) as "pull_requests!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.active_time.total'
                                   THEN om.value ELSE 0 END), 0) as "active_time_secs!: f64",
                 COALESCE(SUM(CASE WHEN om.metric_name = 'claude_code.cost.usage'
                                   THEN om.value ELSE 0 END), 0) as "cost_usd!: f64"
               FROM otel_metrics om
               JOIN tasks t ON t.id = om.task_id
               WHERE om.task_id IS NOT NULL
               GROUP BY om.task_id, om.session_id, t.title
               ORDER BY MAX(om.created_at) DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
