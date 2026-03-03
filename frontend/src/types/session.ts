export type SessionStatus = 'pending' | 'running' | 'completed' | 'failed' | 'stopped';

export interface Session {
  id: string;
  task_id: string;
  status: SessionStatus;
  started_at: string;
  ended_at: string | null;
  error_message: string | null;
  claude_session_id?: string | null;
}

export interface QueueStatus {
  pending: number;
  running: number;
  completed: number;
  failed: number;
}
