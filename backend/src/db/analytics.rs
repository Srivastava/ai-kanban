use crate::models::{
    AnalyticsOverview, DailyTokens, WeeklyTokens, MonthlyTokens,
    TaskTokens, SessionTokens, ToolTokens, LanguageTokens,
    EfficiencyRow, SessionTimelineEvent,
};
use anyhow::Result;
use sqlx::{SqlitePool, sqlite::SqliteRow, Row};
use tracing::{debug, instrument};

/// Pricing constants for Claude Sonnet (as of 2024)
const INPUT_PRICE_PER_MILLION: f64 = 3.0;
const OUTPUT_PRICE_PER_MILLION: f64 = 15.0;

#[derive(Clone)]
pub struct AnalyticsRepository {
    pool: SqlitePool,
}

impl AnalyticsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get overall analytics summary
    #[instrument(skip(self))]
    pub async fn overview(&self) -> Result<AnalyticsOverview> {
        debug!("Fetching analytics overview");

        // Get total tokens
        let totals: SqliteRow = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as input_tokens,
                COALESCE(SUM(output_tokens), 0) as output_tokens
            FROM token_events
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_input: i64 = totals.get("input_tokens");
        let total_output: i64 = totals.get("output_tokens");

        // Calculate estimated cost
        let estimated_cost_usd = (total_input as f64 / 1_000_000.0) * INPUT_PRICE_PER_MILLION
            + (total_output as f64 / 1_000_000.0) * OUTPUT_PRICE_PER_MILLION;

        // Get unique sessions count
        let sessions: SqliteRow = sqlx::query(
            "SELECT COUNT(DISTINCT session_id) as count FROM token_events"
        )
        .fetch_one(&self.pool)
        .await?;

        let session_count: i64 = sessions.get("count");

        // Get unique tasks count
        let tasks: SqliteRow = sqlx::query(
            "SELECT COUNT(DISTINCT task_id) as count FROM token_events"
        )
        .fetch_one(&self.pool)
        .await?;

        let task_count: i64 = tasks.get("count");

        // Get active sessions today
        let active_today: SqliteRow = sqlx::query(
            "SELECT COUNT(DISTINCT session_id) as count FROM token_events WHERE DATE(timestamp) = DATE('now')"
        )
        .fetch_one(&self.pool)
        .await?;

        let active_sessions_today: i64 = active_today.get("count");

        Ok(AnalyticsOverview {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_sessions: session_count,
            total_tasks_with_sessions: task_count,
            estimated_cost_usd,
            active_sessions_today,
        })
    }

    /// Get daily token usage for the last N days
    #[instrument(skip(self))]
    pub async fn daily_tokens(&self, days: i64) -> Result<Vec<DailyTokens>> {
        debug!(days = days, "Fetching daily tokens");

        let rows = sqlx::query(
            r#"
            SELECT
                DATE(timestamp) as date,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens
            FROM token_events
            WHERE DATE(timestamp) >= DATE('now', ?)
            GROUP BY DATE(timestamp)
            ORDER BY date ASC
            "#
        )
        .bind(format!("-{} days", days))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| DailyTokens {
                date: row.get("date"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
            })
            .collect())
    }

    /// Get weekly token usage for the last N weeks
    #[instrument(skip(self))]
    pub async fn weekly_tokens(&self, weeks: i64) -> Result<Vec<WeeklyTokens>> {
        debug!(weeks = weeks, "Fetching weekly tokens");

        let rows = sqlx::query(
            r#"
            SELECT
                DATE(timestamp, 'weekday 0', '-6 days') as week_start,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens
            FROM token_events
            WHERE DATE(timestamp) >= DATE('now', 'weekday 0', ?)
            GROUP BY week_start
            ORDER BY week_start ASC
            "#
        )
        .bind(format!("-{} days", weeks * 7))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| WeeklyTokens {
                week_start: row.get("week_start"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
            })
            .collect())
    }

    /// Get monthly token usage for the last N months
    #[instrument(skip(self))]
    pub async fn monthly_tokens(&self, months: i64) -> Result<Vec<MonthlyTokens>> {
        debug!(months = months, "Fetching monthly tokens");

        let rows = sqlx::query(
            r#"
            SELECT
                strftime('%Y-%m', timestamp) as month,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens
            FROM token_events
            WHERE DATE(timestamp) >= DATE('now', ?)
            GROUP BY month
            ORDER BY month ASC
            "#
        )
        .bind(format!("-{} months", months))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| MonthlyTokens {
                month: row.get("month"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
            })
            .collect())
    }

    /// Get token usage aggregated by task
    #[instrument(skip(self))]
    pub async fn tokens_by_task(&self) -> Result<Vec<TaskTokens>> {
        debug!("Fetching tokens by task");

        let rows = sqlx::query(
            r#"
            SELECT
                te.task_id,
                COALESCE(t.title, 'Unknown Task') as task_title,
                SUM(te.input_tokens) as input_tokens,
                SUM(te.output_tokens) as output_tokens
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            GROUP BY te.task_id, t.title
            ORDER BY (SUM(te.input_tokens) + SUM(te.output_tokens)) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let input: i64 = row.get("input_tokens");
                let output: i64 = row.get("output_tokens");
                TaskTokens {
                    task_id: row.get("task_id"),
                    task_title: row.get("task_title"),
                    input_tokens: input,
                    output_tokens: output,
                    total_tokens: input + output,
                }
            })
            .collect())
    }

    /// Get token usage aggregated by session
    #[instrument(skip(self))]
    pub async fn tokens_by_session(&self) -> Result<Vec<SessionTokens>> {
        debug!("Fetching tokens by session");

        let rows = sqlx::query(
            r#"
            SELECT
                te.session_id,
                COALESCE(t.title, 'Unknown Task') as task_title,
                SUM(te.input_tokens) as input_tokens,
                SUM(te.output_tokens) as output_tokens,
                MIN(te.timestamp) as started_at
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            GROUP BY te.session_id, t.title
            ORDER BY started_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let input: i64 = row.get("input_tokens");
                let output: i64 = row.get("output_tokens");
                let started_at: Option<String> = row.get("started_at");
                SessionTokens {
                    session_id: row.get("session_id"),
                    task_title: row.get("task_title"),
                    input_tokens: input,
                    output_tokens: output,
                    total_tokens: input + output,
                    started_at,
                }
            })
            .collect())
    }

    /// Get token usage aggregated by tool
    #[instrument(skip(self))]
    pub async fn tokens_by_tool(&self) -> Result<Vec<ToolTokens>> {
        debug!("Fetching tokens by tool");

        let rows = sqlx::query(
            r#"
            SELECT
                COALESCE(tool_name, 'unknown') as tool_name,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                COUNT(*) as call_count
            FROM token_events
            WHERE tool_name IS NOT NULL
            GROUP BY tool_name
            ORDER BY (SUM(input_tokens) + SUM(output_tokens)) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ToolTokens {
                tool_name: row.get("tool_name"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                call_count: row.get("call_count"),
            })
            .collect())
    }

    /// Get token usage aggregated by file extension (language)
    #[instrument(skip(self))]
    pub async fn tokens_by_language(&self) -> Result<Vec<LanguageTokens>> {
        debug!("Fetching tokens by language");

        let rows = sqlx::query(
            r#"
            SELECT
                COALESCE(file_ext, 'unknown') as file_ext,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                COUNT(*) as call_count
            FROM token_events
            WHERE file_ext IS NOT NULL
            GROUP BY file_ext
            ORDER BY (SUM(input_tokens) + SUM(output_tokens)) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| LanguageTokens {
                file_ext: row.get("file_ext"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                call_count: row.get("call_count"),
            })
            .collect())
    }

    /// Get token efficiency metrics (tokens per line written, tokens per project LOC)
    #[instrument(skip(self))]
    pub async fn token_efficiency(&self) -> Result<Vec<EfficiencyRow>> {
        debug!("Fetching token efficiency");

        let rows = sqlx::query(
            r#"
            SELECT
                te.task_id,
                COALESCE(t.title, 'Unknown Task') as task_title,
                SUM(te.input_tokens) + SUM(te.output_tokens) as total_tokens,
                COALESCE(SUM(sm.lines_written), 0) as lines_written,
                COALESCE(MAX(sm.project_loc), 0) as project_loc
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            LEFT JOIN session_metrics sm ON te.session_id = sm.session_id
            GROUP BY te.task_id, t.title
            HAVING total_tokens > 0
            ORDER BY total_tokens DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let total: i64 = row.get("total_tokens");
                let lines: i64 = row.get("lines_written");
                let loc: i64 = row.get("project_loc");

                let tokens_per_line = if lines > 0 {
                    Some(total as f64 / lines as f64)
                } else {
                    None
                };

                let tokens_per_loc = if loc > 0 {
                    Some(total as f64 / loc as f64)
                } else {
                    None
                };

                EfficiencyRow {
                    task_id: row.get("task_id"),
                    task_title: row.get("task_title"),
                    total_tokens: total,
                    lines_written: lines,
                    project_loc: loc,
                    tokens_per_line,
                    tokens_per_loc,
                }
            })
            .collect())
    }

    /// Get session timeline events with cumulative token totals
    #[instrument(skip(self))]
    pub async fn session_timeline(&self, session_id: &str) -> Result<Vec<SessionTimelineEvent>> {
        debug!(session_id = session_id, "Fetching session timeline");

        let rows = sqlx::query(
            r#"
            SELECT
                sequence_no,
                event_type,
                tool_name,
                input_tokens,
                output_tokens,
                timestamp
            FROM token_events
            WHERE session_id = ?
            ORDER BY sequence_no ASC, id ASC
            "#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut cumulative = 0i64;
        let timeline: Vec<SessionTimelineEvent> = rows
            .into_iter()
            .map(|row| {
                let input: i64 = row.get("input_tokens");
                let output: i64 = row.get("output_tokens");
                cumulative += input + output;
                SessionTimelineEvent {
                    sequence_no: row.get("sequence_no"),
                    event_type: row.get("event_type"),
                    tool_name: row.get("tool_name"),
                    input_tokens: input,
                    output_tokens: output,
                    cumulative_total: cumulative,
                    timestamp: row.get("timestamp"),
                }
            })
            .collect();

        Ok(timeline)
    }
}
