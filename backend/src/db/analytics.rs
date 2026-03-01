use crate::models::{
    AnalyticsOverview, BurnRate, CostByTask, DailyTokens, WeeklyTokens, MonthlyTokens,
    SessionSummary, TaskTokens, SessionTokens, ToolTokens, LanguageTokens,
    EfficiencyRow, SessionTimelineEvent, TokensByStage, UsageWindows,
};
use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
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

    /// Get token usage in the last 5 hours and this calendar week, plus reset times.
    #[instrument(skip(self))]
    pub async fn usage_windows(&self, limit_5hr: i64, limit_week: i64) -> Result<UsageWindows> {
        debug!("Fetching usage windows");

        // --- 5-hour window ---
        let row_5hr: SqliteRow = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(input_tokens + output_tokens), 0) as tokens,
                MIN(timestamp) as earliest
            FROM token_events
            WHERE timestamp >= datetime('now', '-5 hours')
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let tokens_5hr: i64 = row_5hr.get("tokens");
        let earliest_5hr: Option<String> = row_5hr.get("earliest");

        // Reset = earliest event in window + 5 hours
        let reset_5hr = earliest_5hr.and_then(|ts| {
            chrono::DateTime::parse_from_rfc3339(&ts)
                .ok()
                .map(|dt| (dt + Duration::hours(5)).to_rfc3339())
        });

        // --- Weekly window (Mon 00:00 UTC → next Mon 00:00 UTC) ---
        let now = Utc::now();
        let days_from_monday = now.weekday().num_days_from_monday() as i64;
        let week_start = (now - Duration::days(days_from_monday))
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        let row_week: SqliteRow = sqlx::query(
            r#"
            SELECT COALESCE(SUM(input_tokens + output_tokens), 0) as tokens
            FROM token_events
            WHERE DATE(timestamp) >= ?
            "#,
        )
        .bind(&week_start)
        .fetch_one(&self.pool)
        .await?;

        let tokens_week: i64 = row_week.get("tokens");

        // Reset = next Monday 00:00 UTC
        let days_until_next_monday = 7 - days_from_monday;
        let reset_week_dt = now + Duration::days(days_until_next_monday);
        let reset_week = reset_week_dt
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc().to_rfc3339())
            .unwrap_or_default();

        Ok(UsageWindows {
            tokens_5hr,
            tokens_week,
            limit_5hr,
            limit_week,
            reset_5hr,
            reset_week,
        })
    }

    #[instrument(skip(self))]
    pub async fn cost_by_task(&self) -> Result<Vec<CostByTask>> {
        debug!("Fetching cost by task");
        let rows = sqlx::query(
            r#"
            SELECT
                te.task_id,
                COALESCE(t.title, 'Unknown Task') as task_title,
                SUM(te.input_tokens) as input_tokens,
                SUM(te.output_tokens) as output_tokens,
                CAST(
                    SUM(te.input_tokens) * 3.0 / 1000000.0
                    + SUM(te.output_tokens) * 15.0 / 1000000.0
                AS REAL) as cost_usd
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            GROUP BY te.task_id, t.title
            ORDER BY cost_usd DESC
            LIMIT 20
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| CostByTask {
            task_id: row.get("task_id"),
            task_title: row.get("task_title"),
            input_tokens: row.get("input_tokens"),
            output_tokens: row.get("output_tokens"),
            cost_usd: row.get("cost_usd"),
        }).collect())
    }

    #[instrument(skip(self))]
    pub async fn tokens_by_stage(&self) -> Result<Vec<TokensByStage>> {
        debug!("Fetching tokens by stage");
        let rows = sqlx::query(
            r#"
            SELECT
                t.stage,
                SUM(te.input_tokens) as input_tokens,
                SUM(te.output_tokens) as output_tokens
            FROM token_events te
            JOIN tasks t ON te.task_id = t.id
            GROUP BY t.stage
            ORDER BY t.stage ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| TokensByStage {
            stage: row.get("stage"),
            input_tokens: row.get("input_tokens"),
            output_tokens: row.get("output_tokens"),
        }).collect())
    }

    #[instrument(skip(self))]
    pub async fn session_summary(&self) -> Result<SessionSummary> {
        debug!("Fetching session summary");
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_sessions,
                CAST(COALESCE(AVG(session_total), 0) AS REAL) as avg_tokens_per_session,
                COALESCE(MAX(session_total), 0) as max_tokens_per_session,
                CAST(
                    COALESCE(SUM(input_tokens), 0) * 3.0 / 1000000.0
                    + COALESCE(SUM(output_tokens), 0) * 15.0 / 1000000.0
                AS REAL) as total_cost_usd
            FROM (
                SELECT
                    session_id,
                    SUM(input_tokens) as input_tokens,
                    SUM(output_tokens) as output_tokens,
                    SUM(input_tokens + output_tokens) as session_total
                FROM token_events
                GROUP BY session_id
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(SessionSummary {
            total_sessions: row.get("total_sessions"),
            avg_tokens_per_session: row.get("avg_tokens_per_session"),
            max_tokens_per_session: row.get("max_tokens_per_session"),
            total_cost_usd: row.get("total_cost_usd"),
        })
    }

    #[instrument(skip(self))]
    pub async fn burn_rate(&self) -> Result<BurnRate> {
        debug!("Fetching burn rate");
        let row = sqlx::query(
            r#"
            SELECT
                CAST(COALESCE(SUM(input_tokens + output_tokens), 0) AS REAL) as tokens_last_hour,
                CAST(COALESCE(SUM(input_tokens + output_tokens), 0) / 60.0 AS REAL) as tokens_per_minute
            FROM token_events
            WHERE timestamp >= datetime('now', '-1 hour')
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(BurnRate {
            tokens_last_hour: row.get("tokens_last_hour"),
            tokens_per_minute: row.get("tokens_per_minute"),
        })
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
