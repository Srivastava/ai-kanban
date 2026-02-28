use serde::Serialize;

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
