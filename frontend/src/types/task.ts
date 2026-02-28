export type Stage = 'backlog' | 'planning' | 'ready' | 'in_progress' | 'review' | 'done';

export interface Task {
  id: string;
  title: string;
  description: string | null;
  context: string | null;
  stage: Stage;
  project_path: string;
  session_id: string | null;
  priority: number;
  created_at: string;
  updated_at: string;
}

export interface CreateTask {
  title: string;
  description?: string;
  project_path: string;
}

export interface UpdateTask {
  title?: string;
  description?: string | null;
  context?: string | null;
  stage?: Stage;
  priority?: number;
}
