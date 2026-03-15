use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_sessions: i64,
    pub total_tasks_with_sessions: i64,
    pub estimated_cost_usd: f64,
    pub active_sessions_today: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyTokens {
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct WeeklyTokens {
    pub week_start: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct MonthlyTokens {
    pub month: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct TaskTokens {
    pub task_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionTokens {
    pub session_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub started_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolTokens {
    pub tool_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Serialize)]
pub struct LanguageTokens {
    pub file_ext: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Serialize)]
pub struct EfficiencyRow {
    pub task_id: String,
    pub task_title: String,
    pub total_tokens: i64,
    pub lines_written: i64,
    pub project_loc: i64,
    pub tokens_per_line: Option<f64>,
    pub tokens_per_loc: Option<f64>,
}

/// Token usage summary for rate-limit windows (5-hour and weekly).
/// `limit_*` fields are 0 when not configured (no limit known).
#[derive(Debug, Serialize, Deserialize)]
pub struct UsageWindows {
    pub tokens_5hr: i64,
    pub tokens_week: i64,
    pub limit_5hr: i64,
    pub limit_week: i64,
    /// ISO-8601 timestamp when the current 5-hr window resets (null if no usage)
    pub reset_5hr: Option<String>,
    /// ISO-8601 timestamp when the weekly window resets (always next Monday 00:00 UTC)
    pub reset_week: String,
}

#[derive(Debug, Serialize)]
pub struct SessionTimelineEvent {
    pub sequence_no: i64,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cumulative_total: i64,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct TaskTimelineEvent {
    pub claude_session_id: String,
    pub sequence_no: i64,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cumulative_total: i64,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct CostByTask {
    pub task_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct TokensByStage {
    pub stage: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub total_sessions: i64,
    pub avg_tokens_per_session: f64,
    pub max_tokens_per_session: i64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct BurnRate {
    pub tokens_last_hour: f64,
    pub tokens_per_minute: f64,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub id: String,
    pub task_id: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub claude_session_id: Option<String>,
    pub error_message: Option<String>,
    pub duration_secs: Option<i64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct PlanTier {
    pub tier: String,
    pub limit_5hr: i64,
    pub limit_week: i64,
}

#[derive(Debug, Serialize)]
pub struct RoiMetrics {
    pub cost_per_commit:   Option<f64>,
    pub cost_per_pr:       Option<f64>,
    pub cost_per_loc:      Option<f64>,
    pub total_commits:     i64,
    pub total_prs:         i64,
    pub total_loc:         i64,
    pub total_active_time_secs: f64,
    pub avg_session_duration_secs: f64,
    pub total_cost_usd:    f64,
}
