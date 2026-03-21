export interface AnalyticsOverview {
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_creation_tokens: number;
  total_cache_read_tokens: number;
  total_sessions: number;
  total_tasks_with_sessions: number;
  estimated_cost_usd: number;
  active_sessions_today: number;
}

export interface DailyTokens {
  date: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
}

export interface WeeklyTokens {
  week_start: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
}

export interface MonthlyTokens {
  month: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
}

export interface TaskTokens {
  task_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  total_tokens: number;
}

export interface SessionTokens {
  session_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  total_tokens: number;
  started_at: string | null;
}

export interface ToolTokens {
  tool_name: string;
  input_tokens: number;
  output_tokens: number;
  call_count: number;
}

export interface LanguageTokens {
  file_ext: string;
  input_tokens: number;
  output_tokens: number;
  call_count: number;
}

export interface EfficiencyRow {
  task_id: string;
  task_title: string;
  total_tokens: number;
  lines_written: number;
  project_loc: number;
  tokens_per_line: number | null;
  tokens_per_loc: number | null;
}

export interface SessionTimelineEvent {
  sequence_no: number;
  event_type: string;
  tool_name: string | null;
  input_tokens: number;
  output_tokens: number;
  cumulative_total: number;
  timestamp: string;
}

export interface TaskTimelineEvent {
  claude_session_id: string;
  sequence_no: number;
  event_type: string;
  tool_name: string | null;
  input_tokens: number;
  output_tokens: number;
  cumulative_total: number;
  timestamp: string;
}

export interface UsageWindows {
  tokens_5hr: number;
  tokens_week: number;
  limit_5hr: number;
  limit_week: number;
  reset_5hr: string | null;
  reset_week: string | null;
  no_data: boolean;
}

export interface CostByTask {
  task_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  cost_usd: number;
}

export interface TokensByStage {
  stage: string;
  input_tokens: number;
  output_tokens: number;
}

export interface SessionSummary {
  total_sessions: number;
  avg_tokens_per_session: number;
  max_tokens_per_session: number;
  total_cost_usd: number;
}

export interface BurnRate {
  tokens_last_hour: number;
  tokens_per_minute: number;
}

export interface SessionDetail {
  id: string;
  task_id: string;
  status: string;
  started_at: string;
  ended_at: string | null;
  claude_session_id: string | null;
  error_message: string | null;
  duration_secs: number | null;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
}

export interface DevActivityRow {
  task_id: string;
  task_title: string;
  session_count: number;
  lines_added: number;
  lines_deleted: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  cost_usd: number;
}

export interface PlanTier {
  tier: string;
  limit_5hr: number;
  limit_week: number;
}

export interface RoiMetrics {
  cost_per_commit: number | null;
  cost_per_pr: number | null;
  cost_per_loc: number | null;
  total_commits: number;
  total_prs: number;
  total_loc: number;
  total_active_time_secs: number;
  avg_session_duration_secs: number;
  total_cost_usd: number;
}

export interface ContextWindowUsage {
  session_id: string;
  task_title: string;
  tokens_in_window: number;
  context_limit: number;
  pct_used: number;
}

export interface LocHistoryEntry {
  session_id: string;
  session_index: number;
  project_loc: number;
  started_at: string;
}

export interface HeatmapEntry {
  date: string;   // "YYYY-MM-DD"
  tokens: number;
}

export interface HourlyEntry {
  hour: number;   // 0–23
  tokens: number;
}

// Session-scoped tool breakdown uses the same shape as ToolTokens
export type SessionToolTokens = ToolTokens;
