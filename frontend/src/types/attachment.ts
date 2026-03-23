export interface TaskAttachment {
  id: string;
  task_id: string;
  filename: string;
  mime_type: string;
  size_bytes: number;
  created_at: string;
}
