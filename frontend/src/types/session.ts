export type SessionStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface Session {
  id: string;
  task_id: string;
  status: SessionStatus;
  result: string | null;
  error: string | null;
  created_at: string;
  updated_at: string;
}

export interface QueueStatus {
  pending: number;
  running: number;
  completed: number;
  failed: number;
}
