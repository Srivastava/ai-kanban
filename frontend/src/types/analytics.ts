export interface AnalyticsOverview {
  total_input_tokens: number;
  total_output_tokens: number;
  total_sessions: number;
  total_tasks_with_sessions: number;
  estimated_cost_usd: number;
  active_sessions_today: number;
}

export interface DailyTokens {
  date: string;
  input_tokens: number;
  output_tokens: number;
}

export interface WeeklyTokens {
  week_start: string;
  input_tokens: number;
  output_tokens: number;
}

export interface MonthlyTokens {
  month: string;
  input_tokens: number;
  output_tokens: number;
}

export interface TaskTokens {
  task_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
}

export interface SessionTokens {
  session_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
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
