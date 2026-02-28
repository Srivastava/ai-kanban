import type { Task } from '@/types/task';
import type { LogEntry } from '@/types/log';
import type { AnalyticsOverview } from '@/types/analytics';

export const mockTask: Task = {
  id: 'task-123',
  title: 'Test task',
  description: 'A test task description',
  context: null,
  stage: 'backlog',
  project_path: '/test/project',
  session_id: null,
  priority: 0,
  created_at: '2026-02-27T10:00:00Z',
  updated_at: '2026-02-27T10:00:00Z',
};

export const mockTask2: Task = {
  ...mockTask,
  id: 'task-456',
  title: 'In-progress task',
  stage: 'in_progress',
};

export const mockLog: LogEntry = {
  id: 1,
  timestamp: '2026-02-27T10:00:00Z',
  level: 'INFO',
  message: 'Test log message',
  target: 'test:component',
  source: 'frontend',
  task_id: null,
  session_id: null,
  metadata: null,
  created_at: '2026-02-27T10:00:00Z',
};

export const mockOverview: AnalyticsOverview = {
  total_input_tokens: 150000,
  total_output_tokens: 45000,
  total_sessions: 12,
  total_tasks_with_sessions: 5,
  estimated_cost_usd: 1.125,
  active_sessions_today: 2,
};
