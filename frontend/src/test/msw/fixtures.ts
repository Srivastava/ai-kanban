import type { Task } from '@/types/task';
import type { LogEntry } from '@/types/log';
import type { AnalyticsOverview, CostByTask, TokensByStage, SessionSummary, BurnRate } from '@/types/analytics';

export const mockTask: Task = {
  id: 'task-123',
  title: 'Test task',
  description: 'A test task description',
  instructions: null,
  context: null,
  compressed_context: null,
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

export const mockCostByTask: CostByTask[] = [
  { task_id: 'task-123', task_title: 'Test task', input_tokens: 80000, output_tokens: 24000, cost_usd: 0.6 },
  { task_id: 'task-456', task_title: 'In-progress task', input_tokens: 40000, output_tokens: 10000, cost_usd: 0.27 },
];

export const mockTokensByStage: TokensByStage[] = [
  { stage: 'backlog', input_tokens: 50000, output_tokens: 15000 },
  { stage: 'in_progress', input_tokens: 80000, output_tokens: 24000 },
  { stage: 'done', input_tokens: 20000, output_tokens: 6000 },
];

export const mockSessionSummary: SessionSummary = {
  total_sessions: 12,
  avg_tokens_per_session: 16250,
  max_tokens_per_session: 104000,
  total_cost_usd: 1.125,
};

export const mockBurnRate: BurnRate = {
  tokens_last_hour: 5400,
  tokens_per_minute: 90,
};
