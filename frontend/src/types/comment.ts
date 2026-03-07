export type Author = 'user' | 'claude' | 'litellm';

export interface Comment {
  id: string;
  task_id: string;
  parent_id: string | null;
  author: Author;
  content: string;
  created_at: string;
}

export interface CommentWithReplies {
  id: string;
  task_id: string;
  parent_id: string | null;
  author: Author;
  content: string;
  created_at: string;
  replies: Comment[];
}

export interface CreateComment {
  content: string;
  parent_id?: string;
}
