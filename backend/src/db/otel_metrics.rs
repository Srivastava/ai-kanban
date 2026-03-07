use crate::models::{CreateOtelMetric, DevActivityRow, OtelMetric};
use anyhow::Result;
use sqlx::{Row, SqlitePool};

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

    /// `task_id`: when Some, returns exactly one row for that task; when None, returns all tasks.
    pub async fn dev_activity(&self, task_id: Option<&str>) -> Result<Vec<DevActivityRow>> {
        // Token data from token_events (correlated via session→task FK).
        // Lines and cost from otel_metrics (only populated when OTel telemetry is active).
        // cache_* columns added by migration 011 — omitted here since they are 0 for
        // all data recorded before that migration.
        let rows = sqlx::query(
            r#"SELECT
                 t.id          as task_id,
                 t.title       as task_title,
                 COUNT(DISTINCT s.id) as session_count,
                 COALESCE(oa.lines_added,   0.0) as lines_added,
                 COALESCE(oa.lines_deleted, 0.0) as lines_deleted,
                 COALESCE(oa.cost_usd,      0.0) as cost_usd,
                 COALESCE(ta.input_tokens,  0.0) as input_tokens,
                 COALESCE(ta.output_tokens, 0.0) as output_tokens
               FROM tasks t
               JOIN sessions s ON s.task_id = t.id
               LEFT JOIN (
                 SELECT
                   task_id,
                   CAST(SUM(CASE WHEN metric_name = 'claude_code.lines_of_code.count'
                             AND json_extract(attributes, '$.type') = 'added'
                            THEN value ELSE 0.0 END) AS REAL) as lines_added,
                   CAST(SUM(CASE WHEN metric_name = 'claude_code.lines_of_code.count'
                             AND json_extract(attributes, '$.type') = 'removed'
                            THEN value ELSE 0.0 END) AS REAL) as lines_deleted,
                   CAST(SUM(CASE WHEN metric_name = 'claude_code.cost.usage'
                            THEN value ELSE 0.0 END) AS REAL) as cost_usd
                 FROM otel_metrics
                 WHERE task_id IS NOT NULL
                 GROUP BY task_id
               ) oa ON oa.task_id = t.id
               LEFT JOIN (
                 SELECT
                   task_id,
                   CAST(SUM(input_tokens)  AS REAL) as input_tokens,
                   CAST(SUM(output_tokens) AS REAL) as output_tokens
                 FROM token_events
                 WHERE task_id IS NOT NULL
                 GROUP BY task_id
               ) ta ON ta.task_id = t.id
               WHERE (? IS NULL OR t.id = ?)
               GROUP BY t.id, t.title
               ORDER BY MAX(s.started_at) DESC"#
        )
        .bind(task_id)
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| DevActivityRow {
            task_id:       row.get("task_id"),
            task_title:    row.get("task_title"),
            session_count: row.get("session_count"),
            lines_added:   row.get::<f64, _>("lines_added"),
            lines_deleted: row.get::<f64, _>("lines_deleted"),
            input_tokens:  row.get::<f64, _>("input_tokens"),
            output_tokens: row.get::<f64, _>("output_tokens"),
            cost_usd:      row.get::<f64, _>("cost_usd"),
        }).collect())
    }
}
