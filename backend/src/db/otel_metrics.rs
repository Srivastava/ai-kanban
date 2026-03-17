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

    /// Sum all values for a given metric_name across all stored data points.
    pub async fn sum_metric(&self, metric_name: &str) -> Result<f64> {
        let row = sqlx::query!(
            r#"SELECT COALESCE(SUM(value), 0.0) as "total: f64" FROM otel_metrics WHERE metric_name = ?"#,
            metric_name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.total)
    }

    /// Sum lines_of_code.count by type attribute, returns (added, removed).
    pub async fn sum_lines_of_code(&self) -> Result<(f64, f64)> {
        let row = sqlx::query!(
            r#"SELECT
               COALESCE(SUM(CASE WHEN json_extract(attributes, '$.type') = 'added'   THEN value ELSE 0 END), 0.0) as "added: f64",
               COALESCE(SUM(CASE WHEN json_extract(attributes, '$.type') = 'removed' THEN value ELSE 0 END), 0.0) as "removed: f64"
               FROM otel_metrics WHERE metric_name = 'claude_code.lines_of_code.count'"#
        )
        .fetch_one(&self.pool)
        .await?;
        Ok((row.added, row.removed))
    }

    /// `task_id`: when Some, returns exactly one row for that task; when None, returns all tasks.
    pub async fn dev_activity(&self, task_id: Option<&str>) -> Result<Vec<DevActivityRow>> {
        // Lines-of-code source: session_metrics.project_loc captured at every session start.
        // current_loc  = LOC at the last session (project as it stands now)
        // baseline_loc = earliest non-zero LOC recorded (before AI started working)
        // loc_written  = current_loc - baseline_loc (net lines grown under AI's hand)
        //
        // OTel lines_of_code.count is only available when OTel telemetry was active AND
        // the session was correlated — typically a tiny fraction of sessions, so unreliable.
        let rows = sqlx::query(
            r#"SELECT
                 t.id          as task_id,
                 t.title       as task_title,
                 COUNT(DISTINCT s.id) as session_count,
                 -- Project LOC growth: current size minus earliest recorded size
                 COALESCE(sm_agg.current_loc,  0) as current_loc,
                 COALESCE(sm_agg.baseline_loc, 0) as baseline_loc,
                 CAST(COALESCE(sm_agg.current_loc, 0) - COALESCE(sm_agg.baseline_loc, 0) AS REAL) as loc_written,
                 -- Cost from OTel where available
                 COALESCE(oa.cost_usd, 0.0) as cost_usd,
                 -- Token totals from token_events (well-correlated via session FK)
                 COALESCE(ta.input_tokens,           0.0) as input_tokens,
                 COALESCE(ta.output_tokens,          0.0) as output_tokens,
                 COALESCE(ta.cache_creation_tokens,  0.0) as cache_creation_tokens,
                 COALESCE(ta.cache_read_tokens,      0.0) as cache_read_tokens
               FROM tasks t
               JOIN sessions s ON s.task_id = t.id
               -- Session metrics: get current LOC and baseline (first non-zero LOC)
               LEFT JOIN (
                 SELECT
                   s2.task_id,
                   MAX(sm2.project_loc) as current_loc,
                   MIN(CASE WHEN sm2.project_loc > 0 THEN sm2.project_loc END) as baseline_loc
                 FROM session_metrics sm2
                 JOIN sessions s2 ON s2.id = sm2.session_id
                 GROUP BY s2.task_id
               ) sm_agg ON sm_agg.task_id = t.id
               -- OTel: cost only (lines_of_code unreliable — only partial sessions)
               LEFT JOIN (
                 SELECT
                   task_id,
                   CAST(SUM(CASE WHEN metric_name = 'claude_code.cost.usage'
                            THEN value ELSE 0.0 END) AS REAL) as cost_usd
                 FROM otel_metrics
                 WHERE task_id IS NOT NULL
                 GROUP BY task_id
               ) oa ON oa.task_id = t.id
               -- Token totals
               LEFT JOIN (
                 SELECT
                   task_id,
                   CAST(SUM(input_tokens)           AS REAL) as input_tokens,
                   CAST(SUM(output_tokens)          AS REAL) as output_tokens,
                   CAST(SUM(cache_creation_tokens)  AS REAL) as cache_creation_tokens,
                   CAST(SUM(cache_read_tokens)      AS REAL) as cache_read_tokens
                 FROM token_events
                 WHERE event_type = 'assistant' AND task_id IS NOT NULL
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
            task_id:                row.get("task_id"),
            task_title:             row.get("task_title"),
            session_count:          row.get("session_count"),
            lines_added:            row.get::<f64, _>("loc_written"),
            lines_deleted:          0.0, // net growth metric — deletions absorbed into loc_written
            input_tokens:           row.get::<f64, _>("input_tokens"),
            output_tokens:          row.get::<f64, _>("output_tokens"),
            cache_creation_tokens:  row.get::<f64, _>("cache_creation_tokens"),
            cache_read_tokens:      row.get::<f64, _>("cache_read_tokens"),
            cost_usd:               row.get::<f64, _>("cost_usd"),
        }).collect())
    }
}
