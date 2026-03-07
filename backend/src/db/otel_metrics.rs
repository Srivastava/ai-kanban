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

    pub async fn dev_activity(&self) -> Result<Vec<DevActivityRow>> {
        // Token data (input/output) comes from token_events — better task correlation.
        // Lines and cost come from otel_metrics (only populated when OTel telemetry is active).
        let rows = sqlx::query(
            r#"SELECT
                 t.id          as task_id,
                 t.title       as task_title,
                 COUNT(DISTINCT s.id) as session_count,
                 -- Lines from OTel (claude_code.lines_of_code.count with type attribute)
                 COALESCE(oa.lines_added,   0.0) as lines_added,
                 COALESCE(oa.lines_deleted, 0.0) as lines_deleted,
                 -- Cost from OTel (claude_code.cost.usage)
                 COALESCE(oa.cost_usd,      0.0) as cost_usd,
                 -- Token counts from token_events (well-correlated via session/task join)
                 COALESCE(ta.input_tokens,          0.0) as input_tokens,
                 COALESCE(ta.output_tokens,         0.0) as output_tokens,
                 COALESCE(ta.cache_read_tokens,     0.0) as cache_read_tokens,
                 COALESCE(ta.cache_creation_tokens, 0.0) as cache_creation_tokens
               FROM tasks t
               JOIN sessions s ON s.task_id = t.id
               -- OTel aggregates per task (lines + cost)
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
               -- Token aggregates per task from token_events
               LEFT JOIN (
                 SELECT
                   task_id,
                   CAST(SUM(input_tokens)          AS REAL) as input_tokens,
                   CAST(SUM(output_tokens)          AS REAL) as output_tokens,
                   CAST(SUM(cache_read_tokens)      AS REAL) as cache_read_tokens,
                   CAST(SUM(cache_creation_tokens)  AS REAL) as cache_creation_tokens
                 FROM token_events
                 WHERE task_id IS NOT NULL
                 GROUP BY task_id
               ) ta ON ta.task_id = t.id
               GROUP BY t.id, t.title
               ORDER BY MAX(s.started_at) DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| DevActivityRow {
            task_id:              row.get("task_id"),
            task_title:           row.get("task_title"),
            session_count:        row.get("session_count"),
            lines_added:          row.get::<f64, _>("lines_added"),
            lines_deleted:        row.get::<f64, _>("lines_deleted"),
            input_tokens:         row.get::<f64, _>("input_tokens"),
            output_tokens:        row.get::<f64, _>("output_tokens"),
            cache_read_tokens:    row.get::<f64, _>("cache_read_tokens"),
            cache_creation_tokens: row.get::<f64, _>("cache_creation_tokens"),
            cost_usd:             row.get::<f64, _>("cost_usd"),
        }).collect())
    }
}
