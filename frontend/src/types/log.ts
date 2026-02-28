export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
export type LogSource = 'frontend' | 'backend';

export interface LogEntry {
  id: number;
  timestamp: string;   // ISO 8601
  level: LogLevel;
  message: string;
  target: string | null;
  source: LogSource;
  task_id: string | null;
  session_id: string | null;
  metadata: string | null;  // JSON string
  created_at: string;
}

export interface LogFilter {
  level?: LogLevel;
  source?: LogSource;
  task_id?: string;
  session_id?: string;
  search?: string;     // client-side text filter
}
