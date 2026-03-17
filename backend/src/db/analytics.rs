use crate::models::{
    AnalyticsOverview, BurnRate, CostByTask, DailyTokens, WeeklyTokens, MonthlyTokens,
    SessionSummary, TaskTokens, SessionTokens, ToolTokens, LanguageTokens,
    EfficiencyRow, SessionTimelineEvent, TaskTimelineEvent, TokensByStage, UsageWindows,
};
use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
use std::collections::HashMap;
use sqlx::{SqlitePool, sqlite::SqliteRow, Row};
use tracing::{debug, instrument};

/// Sonnet pricing per million tokens (USD).
/// - Input:         $3.00  (uncached new input)
/// - Output:        $15.00
/// - Cache write:   $3.75  (1.25× input — writing to prompt cache)
/// - Cache read:    $0.30  (0.10× input — reading from prompt cache)
///
/// Configurable via env vars CLAUDE_INPUT_PRICE_PER_MILLION,
/// CLAUDE_OUTPUT_PRICE_PER_MILLION, CLAUDE_CACHE_WRITE_PRICE_PER_MILLION,
/// CLAUDE_CACHE_READ_PRICE_PER_MILLION.
pub struct TokenPrices {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
}

pub fn token_prices() -> TokenPrices {
    let input = std::env::var("CLAUDE_INPUT_PRICE_PER_MILLION")
        .ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(3.0);
    let output = std::env::var("CLAUDE_OUTPUT_PRICE_PER_MILLION")
        .ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(15.0);
    let cache_write = std::env::var("CLAUDE_CACHE_WRITE_PRICE_PER_MILLION")
        .ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(3.75);
    let cache_read = std::env::var("CLAUDE_CACHE_READ_PRICE_PER_MILLION")
        .ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.30);
    TokenPrices { input, output, cache_write, cache_read }
}

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
                COALESCE(SUM(output_tokens), 0) as output_tokens,
                COALESCE(SUM(cache_creation_tokens), 0) as cache_creation_tokens,
                COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens
            FROM token_events
            WHERE event_type = 'assistant'
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_input: i64 = totals.get("input_tokens");
        let total_output: i64 = totals.get("output_tokens");
        let total_cache_creation: i64 = totals.get("cache_creation_tokens");
        let total_cache_read: i64 = totals.get("cache_read_tokens");

        // Calculate estimated cost (configurable via env vars)
        let p = token_prices();
        let estimated_cost_usd = (total_input as f64 / 1_000_000.0) * p.input
            + (total_output as f64 / 1_000_000.0) * p.output
            + (total_cache_creation as f64 / 1_000_000.0) * p.cache_write
            + (total_cache_read as f64 / 1_000_000.0) * p.cache_read;

        // Get unique sessions count
        let sessions: SqliteRow = sqlx::query(
            "SELECT COUNT(DISTINCT session_id) as count FROM token_events WHERE event_type = 'assistant'"
        )
        .fetch_one(&self.pool)
        .await?;

        let session_count: i64 = sessions.get("count");

        // Get unique tasks count
        let tasks: SqliteRow = sqlx::query(
            "SELECT COUNT(DISTINCT task_id) as count FROM token_events WHERE event_type = 'assistant'"
        )
        .fetch_one(&self.pool)
        .await?;

        let task_count: i64 = tasks.get("count");

        // Get active sessions today
        let active_today: SqliteRow = sqlx::query(
            "SELECT COUNT(*) as count FROM sessions WHERE DATE(started_at) = DATE('now')"
        )
        .fetch_one(&self.pool)
        .await?;

        let active_sessions_today: i64 = active_today.get("count");

        Ok(AnalyticsOverview {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cache_creation_tokens: total_cache_creation,
            total_cache_read_tokens: total_cache_read,
            total_sessions: session_count,
            total_tasks_with_sessions: task_count,
            estimated_cost_usd,
            active_sessions_today,
        })
    }

    /// Get daily token usage for the last N days
    #[instrument(skip(self))]
    pub async fn daily_tokens(&self, days: i64, task_id: Option<&str>) -> Result<Vec<DailyTokens>> {
        debug!(days = days, "Fetching daily tokens");

        let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
        let sql = format!(
            r#"
            SELECT
                DATE(timestamp) as date,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                SUM(cache_creation_tokens) as cache_creation_tokens,
                SUM(cache_read_tokens) as cache_read_tokens
            FROM token_events
            WHERE event_type = 'assistant' AND DATE(timestamp) >= DATE('now', ?){task_filter}
            GROUP BY DATE(timestamp)
            ORDER BY date ASC
            "#,
            task_filter = task_filter
        );
        let rows = if let Some(tid) = task_id {
            sqlx::query(&sql)
                .bind(format!("-{} days", days))
                .bind(tid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(&sql)
                .bind(format!("-{} days", days))
                .fetch_all(&self.pool)
                .await?
        };

        Ok(rows
            .into_iter()
            .map(|row| DailyTokens {
                date: row.get("date"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                cache_creation_tokens: row.get("cache_creation_tokens"),
                cache_read_tokens: row.get("cache_read_tokens"),
            })
            .collect())
    }

    /// Get weekly token usage for the last N weeks
    #[instrument(skip(self))]
    pub async fn weekly_tokens(&self, weeks: i64, task_id: Option<&str>) -> Result<Vec<WeeklyTokens>> {
        debug!(weeks = weeks, "Fetching weekly tokens");

        let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
        let sql = format!(
            r#"
            SELECT
                DATE(timestamp, 'weekday 0', '-6 days') as week_start,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                SUM(cache_creation_tokens) as cache_creation_tokens,
                SUM(cache_read_tokens) as cache_read_tokens
            FROM token_events
            WHERE event_type = 'assistant' AND DATE(timestamp) >= DATE('now', 'weekday 0', ?){task_filter}
            GROUP BY week_start
            ORDER BY week_start ASC
            "#,
            task_filter = task_filter
        );
        let rows = if let Some(tid) = task_id {
            sqlx::query(&sql)
                .bind(format!("-{} days", weeks * 7))
                .bind(tid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(&sql)
                .bind(format!("-{} days", weeks * 7))
                .fetch_all(&self.pool)
                .await?
        };

        Ok(rows
            .into_iter()
            .map(|row| WeeklyTokens {
                week_start: row.get("week_start"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                cache_creation_tokens: row.get("cache_creation_tokens"),
                cache_read_tokens: row.get("cache_read_tokens"),
            })
            .collect())
    }

    /// Get monthly token usage for the last N months
    #[instrument(skip(self))]
    pub async fn monthly_tokens(&self, months: i64, task_id: Option<&str>) -> Result<Vec<MonthlyTokens>> {
        debug!(months = months, "Fetching monthly tokens");

        let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
        let sql = format!(
            r#"
            SELECT
                strftime('%Y-%m', timestamp) as month,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                SUM(cache_creation_tokens) as cache_creation_tokens,
                SUM(cache_read_tokens) as cache_read_tokens
            FROM token_events
            WHERE event_type = 'assistant' AND DATE(timestamp) >= DATE('now', ?){task_filter}
            GROUP BY month
            ORDER BY month ASC
            "#,
            task_filter = task_filter
        );
        let rows = if let Some(tid) = task_id {
            sqlx::query(&sql)
                .bind(format!("-{} months", months))
                .bind(tid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(&sql)
                .bind(format!("-{} months", months))
                .fetch_all(&self.pool)
                .await?
        };

        Ok(rows
            .into_iter()
            .map(|row| MonthlyTokens {
                month: row.get("month"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                cache_creation_tokens: row.get("cache_creation_tokens"),
                cache_read_tokens: row.get("cache_read_tokens"),
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
                SUM(te.output_tokens) as output_tokens,
                SUM(te.cache_creation_tokens) as cache_creation_tokens,
                SUM(te.cache_read_tokens) as cache_read_tokens
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            WHERE te.event_type = 'assistant'
            GROUP BY te.task_id
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
                let cache_creation: i64 = row.get("cache_creation_tokens");
                let cache_read: i64 = row.get("cache_read_tokens");
                TaskTokens {
                    task_id: row.get("task_id"),
                    task_title: row.get("task_title"),
                    input_tokens: input,
                    output_tokens: output,
                    cache_creation_tokens: cache_creation,
                    cache_read_tokens: cache_read,
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
                SUM(te.cache_creation_tokens) as cache_creation_tokens,
                SUM(te.cache_read_tokens) as cache_read_tokens,
                MIN(te.timestamp) as started_at
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            WHERE te.event_type = 'assistant'
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
                let cache_creation: i64 = row.get("cache_creation_tokens");
                let cache_read: i64 = row.get("cache_read_tokens");
                let started_at: Option<String> = row.get("started_at");
                SessionTokens {
                    session_id: row.get("session_id"),
                    task_title: row.get("task_title"),
                    input_tokens: input,
                    output_tokens: output,
                    cache_creation_tokens: cache_creation,
                    cache_read_tokens: cache_read,
                    total_tokens: input + output,
                    started_at,
                }
            })
            .collect())
    }

    /// Get token usage aggregated by tool
    #[instrument(skip(self))]
    pub async fn tokens_by_tool(&self, task_id: Option<&str>) -> Result<Vec<ToolTokens>> {
        debug!("Fetching tokens by tool");

        let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
        let sql = format!(
            r#"
            SELECT
                COALESCE(tool_name, 'unknown') as tool_name,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                COUNT(*) as call_count
            FROM token_events
            WHERE event_type = 'assistant' AND tool_name IS NOT NULL{task_filter}
            GROUP BY tool_name
            ORDER BY (SUM(input_tokens) + SUM(output_tokens)) DESC
            "#,
            task_filter = task_filter
        );
        let rows = if let Some(tid) = task_id {
            sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
        } else {
            sqlx::query(&sql).fetch_all(&self.pool).await?
        };

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
    pub async fn tokens_by_language(&self, task_id: Option<&str>) -> Result<Vec<LanguageTokens>> {
        debug!("Fetching tokens by language");

        let task_filter = if task_id.is_some() { " AND task_id = ?" } else { "" };
        let sql = format!(
            r#"
            SELECT
                COALESCE(file_ext, 'unknown') as file_ext,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens,
                COUNT(*) as call_count
            FROM token_events
            WHERE event_type = 'assistant' AND file_ext IS NOT NULL{task_filter}
            GROUP BY file_ext
            ORDER BY (SUM(input_tokens) + SUM(output_tokens)) DESC
            "#,
            task_filter = task_filter
        );
        let rows = if let Some(tid) = task_id {
            sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
        } else {
            sqlx::query(&sql).fetch_all(&self.pool).await?
        };

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
                -- Net LOC growth: current project size minus earliest recorded size.
                -- Uses project_loc snapshots taken at session start (reliable).
                -- lines_written column is never populated so we derive it this way.
                CAST(
                    COALESCE(MAX(sm.project_loc), 0)
                    - COALESCE(MIN(CASE WHEN sm.project_loc > 0 THEN sm.project_loc END), 0)
                AS REAL) as lines_written,
                CAST(COALESCE(MAX(sm.project_loc), 0) AS INTEGER) as project_loc
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            LEFT JOIN session_metrics sm ON te.session_id = sm.session_id
            WHERE te.event_type = 'assistant' AND te.task_id IS NOT NULL
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
                let lines_f: f64 = row.get::<f64, _>("lines_written");
                let lines = lines_f as i64;
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
            WHERE event_type = 'assistant' AND timestamp >= datetime('now', '-5 hours')
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
            WHERE event_type = 'assistant' AND DATE(timestamp) >= ?
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
            reset_week: Some(reset_week),
            // false is correct here: this is the DB/JSONL fallback path and we do have data
            no_data: false,
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
                SUM(te.cache_creation_tokens) as cache_creation_tokens,
                SUM(te.cache_read_tokens) as cache_read_tokens
            FROM token_events te
            LEFT JOIN tasks t ON te.task_id = t.id
            WHERE te.event_type = 'assistant'
            GROUP BY te.task_id, t.title
            LIMIT 20
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let p = token_prices();
        let mut results: Vec<CostByTask> = rows.into_iter().map(|row| {
            let input_tokens: i64 = row.get("input_tokens");
            let output_tokens: i64 = row.get("output_tokens");
            let cache_creation: i64 = row.get("cache_creation_tokens");
            let cache_read: i64 = row.get("cache_read_tokens");
            let cost_usd = (input_tokens as f64 / 1_000_000.0) * p.input
                + (output_tokens as f64 / 1_000_000.0) * p.output
                + (cache_creation as f64 / 1_000_000.0) * p.cache_write
                + (cache_read as f64 / 1_000_000.0) * p.cache_read;
            CostByTask {
                task_id: row.get("task_id"),
                task_title: row.get("task_title"),
                input_tokens,
                output_tokens,
                cache_creation_tokens: cache_creation,
                cache_read_tokens: cache_read,
                cost_usd,
            }
        }).collect();
        results.sort_by(|a, b| b.cost_usd.partial_cmp(&a.cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    #[instrument(skip(self))]
    pub async fn tokens_by_stage(&self, task_id: Option<&str>) -> Result<Vec<TokensByStage>> {
        debug!("Fetching tokens by stage");
        let task_filter = if task_id.is_some() { "AND te.task_id = ?" } else { "" };
        let sql = format!(
            r#"
            SELECT
                COALESCE(
                    (SELECT sh.to_stage FROM stage_history sh
                     WHERE sh.task_id = te.task_id AND sh.moved_at <= te.timestamp
                     ORDER BY sh.moved_at DESC LIMIT 1),
                    (SELECT sh.from_stage FROM stage_history sh
                     WHERE sh.task_id = te.task_id
                     ORDER BY sh.moved_at ASC LIMIT 1),
                    t.stage
                ) as stage,
                SUM(te.input_tokens) as input_tokens,
                SUM(te.output_tokens) as output_tokens
            FROM token_events te
            JOIN tasks t ON te.task_id = t.id
            WHERE te.event_type = 'assistant' {task_filter}
            GROUP BY stage
            ORDER BY stage ASC
            "#,
            task_filter = task_filter
        );
        let rows = if let Some(tid) = task_id {
            sqlx::query(&sql).bind(tid).fetch_all(&self.pool).await?
        } else {
            sqlx::query(&sql).fetch_all(&self.pool).await?
        };
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
                COALESCE(SUM(input_tokens), 0) as total_input,
                COALESCE(SUM(output_tokens), 0) as total_output,
                COALESCE(SUM(cache_creation_tokens), 0) as total_cache_creation,
                COALESCE(SUM(cache_read_tokens), 0) as total_cache_read
            FROM (
                SELECT
                    session_id,
                    SUM(input_tokens) as input_tokens,
                    SUM(output_tokens) as output_tokens,
                    SUM(cache_creation_tokens) as cache_creation_tokens,
                    SUM(cache_read_tokens) as cache_read_tokens,
                    SUM(input_tokens + output_tokens) as session_total
                FROM token_events
                WHERE event_type = 'assistant'
                GROUP BY session_id
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let p = token_prices();
        let total_input: i64 = row.get("total_input");
        let total_output: i64 = row.get("total_output");
        let total_cache_creation: i64 = row.get("total_cache_creation");
        let total_cache_read: i64 = row.get("total_cache_read");
        let total_cost_usd = (total_input as f64 / 1_000_000.0) * p.input
            + (total_output as f64 / 1_000_000.0) * p.output
            + (total_cache_creation as f64 / 1_000_000.0) * p.cache_write
            + (total_cache_read as f64 / 1_000_000.0) * p.cache_read;
        Ok(SessionSummary {
            total_sessions: row.get("total_sessions"),
            avg_tokens_per_session: row.get("avg_tokens_per_session"),
            max_tokens_per_session: row.get("max_tokens_per_session"),
            total_cost_usd,
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
            WHERE event_type = 'assistant' AND timestamp >= datetime('now', '-1 hour')
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
            WHERE event_type = 'assistant' AND session_id = ?
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

    pub async fn roi_metrics(&self, task_id: Option<&str>) -> Result<crate::models::RoiMetrics> {
        // token_prices() is defined in this same file — call directly, no import needed
        let p = token_prices();

        let cost_row = sqlx::query(r#"
            SELECT
                COALESCE(SUM(te.input_tokens), 0)          AS total_input,
                COALESCE(SUM(te.output_tokens), 0)         AS total_output,
                COALESCE(SUM(te.cache_creation_tokens), 0) AS total_cache_creation,
                COALESCE(SUM(te.cache_read_tokens), 0)     AS total_cache_read,
                COUNT(DISTINCT te.session_id)               AS session_count,
                COALESCE(
                    AVG(CASE WHEN s.ended_at IS NOT NULL
                        THEN CAST((julianday(s.ended_at) - julianday(s.started_at)) * 86400.0 AS REAL)
                        END),
                    0.0
                ) AS avg_duration_secs
            FROM token_events te
            JOIN sessions s ON s.id = te.session_id
            WHERE te.event_type = 'assistant' AND (? IS NULL OR te.task_id = ?)
        "#)
        .bind(task_id)
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?;

        let total_input:          f64 = cost_row.get::<i64, _>("total_input")          as f64;
        let total_output:         f64 = cost_row.get::<i64, _>("total_output")         as f64;
        let total_cache_creation: f64 = cost_row.get::<i64, _>("total_cache_creation") as f64;
        let total_cache_read:     f64 = cost_row.get::<i64, _>("total_cache_read")     as f64;
        let avg_duration: f64 = cost_row.get("avg_duration_secs");
        let total_cost = (total_input / 1_000_000.0) * p.input
                       + (total_output / 1_000_000.0) * p.output
                       + (total_cache_creation / 1_000_000.0) * p.cache_write
                       + (total_cache_read / 1_000_000.0) * p.cache_read;

        let otel_row = sqlx::query(r#"
            SELECT
                CAST(COALESCE(SUM(CASE WHEN metric_name = 'claude_code.commit.count'
                             THEN value ELSE 0 END), 0) AS REAL) AS total_commits,
                CAST(COALESCE(SUM(CASE WHEN metric_name = 'claude_code.pull_request.count'
                             THEN value ELSE 0 END), 0) AS REAL) AS total_prs,
                CAST(COALESCE(SUM(CASE WHEN metric_name = 'claude_code.active_time.total'
                             THEN value ELSE 0 END), 0) AS REAL) AS total_active_time
            FROM otel_metrics
            WHERE (? IS NULL OR task_id = ?)
        "#)
        .bind(task_id)
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?;

        let total_commits: i64 = {
            let v: f64 = otel_row.get("total_commits");
            v as i64
        };
        let total_prs: i64 = {
            let v: f64 = otel_row.get("total_prs");
            v as i64
        };
        let total_active: f64 = otel_row.get("total_active_time");

        let loc_row = sqlx::query(r#"
            SELECT
                CAST(
                    COALESCE(MAX(sm.project_loc), 0)
                    - COALESCE(MIN(CASE WHEN sm.project_loc > 0 THEN sm.project_loc END), 0)
                AS REAL) AS net_loc
            FROM session_metrics sm
            JOIN sessions s ON s.id = sm.session_id
            WHERE (? IS NULL OR s.task_id = ?)
        "#)
        .bind(task_id)
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?;

        // SQLite returns arithmetic expressions as REAL — decode as f64, cast to i64
        let total_loc: i64 = {
            let v: f64 = loc_row.get("net_loc");
            v as i64
        };

        Ok(crate::models::RoiMetrics {
            cost_per_commit:  if total_commits > 0 { Some(total_cost / total_commits as f64) } else { None },
            cost_per_pr:      if total_prs > 0     { Some(total_cost / total_prs as f64)     } else { None },
            cost_per_loc:     if total_loc > 0     { Some(total_cost / total_loc as f64)     } else { None },
            total_commits,
            total_prs,
            total_loc,
            total_active_time_secs: total_active,
            avg_session_duration_secs: avg_duration,
            total_cost_usd: total_cost,
        })
    }

    pub async fn context_window_usage(&self) -> Result<Vec<crate::models::ContextWindowUsage>> {
        let context_limit: i64 = std::env::var("CLAUDE_CONTEXT_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(190_000);

        // Fetch all running sessions with their task titles
        let sessions = sqlx::query(
            r#"SELECT s.id AS session_id, t.title AS task_title
               FROM sessions s
               JOIN tasks t ON t.id = s.task_id
               WHERE s.status = 'running'
               ORDER BY s.started_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();

        for row in sessions {
            let session_id: String = row.get("session_id");
            let task_title: String = row.get("task_title");

            // Fetch token events for this session ordered by id ASC
            // tokens_in_window = input_tokens + cache_read_tokens + cache_creation_tokens
            let events = sqlx::query(
                r#"SELECT input_tokens + cache_read_tokens + cache_creation_tokens AS ctx
                   FROM token_events
                   WHERE event_type = 'assistant' AND session_id = ?
                   ORDER BY id ASC"#
            )
            .bind(&session_id)
            .fetch_all(&self.pool)
            .await?;

            // Find last compaction boundary (ctx < 50% of previous)
            let mut boundary_idx = 0usize;
            let ctxs: Vec<i64> = events.iter()
                .map(|r| r.get::<i64, _>("ctx"))
                .collect();

            for i in 1..ctxs.len() {
                if ctxs[i] < ctxs[i - 1] / 2 {
                    boundary_idx = i;
                }
            }

            let tokens_in_window: i64 = ctxs[boundary_idx..].iter().sum();
            // Skip stale/zombie sessions that have no token data
            if tokens_in_window == 0 {
                continue;
            }
            let pct_used = if context_limit > 0 {
                tokens_in_window as f64 / context_limit as f64 * 100.0
            } else {
                0.0
            };

            result.push(crate::models::ContextWindowUsage {
                session_id,
                task_title,
                tokens_in_window,
                context_limit,
                pct_used,
            });
        }

        Ok(result)
    }

    /// All token events for every session of a task, with claude_session_id for grouping.
    /// Cumulative totals are computed per claude_session_id.
    #[instrument(skip(self))]
    pub async fn task_timeline(&self, task_id: &str) -> Result<Vec<TaskTimelineEvent>> {
        debug!(task_id = task_id, "Fetching task timeline");

        let rows = sqlx::query(
            r#"
            SELECT
                COALESCE(s.claude_session_id, te.session_id) AS claude_session_id,
                te.sequence_no,
                te.event_type,
                te.tool_name,
                te.input_tokens,
                te.output_tokens,
                te.timestamp
            FROM token_events te
            LEFT JOIN sessions s ON te.session_id = s.id
            WHERE te.event_type = 'assistant' AND te.task_id = ?
            ORDER BY claude_session_id, te.sequence_no ASC, te.id ASC
            "#
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        let mut cumulatives: HashMap<String, i64> = HashMap::new();
        let events = rows
            .into_iter()
            .map(|row| {
                let csid: String = row.get("claude_session_id");
                let input: i64 = row.get("input_tokens");
                let output: i64 = row.get("output_tokens");
                let cum = cumulatives.entry(csid.clone()).or_insert(0);
                *cum += input + output;
                TaskTimelineEvent {
                    claude_session_id: csid,
                    sequence_no: row.get("sequence_no"),
                    event_type: row.get("event_type"),
                    tool_name: row.get("tool_name"),
                    input_tokens: input,
                    output_tokens: output,
                    cumulative_total: *cum,
                    timestamp: row.get("timestamp"),
                }
            })
            .collect();

        Ok(events)
    }
}
