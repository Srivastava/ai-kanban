export type Stage = 'backlog' | 'planning' | 'ready' | 'in_progress' | 'review' | 'done';

export interface Task {
  id: string;
  title: string;
  description: string | null;
  stage: Stage;
  priority: number;
  created_at: string;
  updated_at: string;
}

export interface CreateTask {
  title: string;
  description?: string;
  stage?: Stage;
  priority?: number;
}

export interface UpdateTask {
  title?: string;
  description?: string | null;
  stage?: Stage;
  priority?: number;
}
