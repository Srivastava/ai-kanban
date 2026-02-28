'use client';

import { Badge } from '@/components/ui/badge';
import { TaskSection } from './task-section';
import { useComments } from '@/hooks/use-comments';
import type { Task, Stage } from '@/types/task';
import type { CommentWithReplies } from '@/types/comment';

const stageColors: Record<Stage, string> = {
  backlog: 'bg-gray-500',
  planning: 'bg-blue-500',
  ready: 'bg-yellow-500',
  in_progress: 'bg-orange-500',
  review: 'bg-purple-500',
  done: 'bg-green-500',
};

const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

// Placeholder for CommentThread component (Task 9)
function CommentThreadPlaceholder({ taskId, comments }: { taskId: string; comments: CommentWithReplies[] }) {
  if (comments.length === 0) {
    return <p className="text-muted-foreground italic">No comments yet. Be the first to comment!</p>;
  }

  return (
    <div className="space-y-4">
      {comments.map((comment) => (
        <div key={comment.id} className="border-l-2 border-muted pl-4">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-medium">{comment.author === 'user' ? 'You' : 'Claude'}</span>
            <span className="text-xs text-muted-foreground">
              {new Date(comment.created_at).toLocaleDateString()}
            </span>
          </div>
          <p className="text-sm">{comment.content}</p>
          {comment.replies.length > 0 && (
            <div className="mt-2 ml-4 space-y-2">
              {comment.replies.map((reply) => (
                <div key={reply.id} className="border-l-2 border-muted pl-4">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-sm font-medium">{reply.author === 'user' ? 'You' : 'Claude'}</span>
                    <span className="text-xs text-muted-foreground">
                      {new Date(reply.created_at).toLocaleDateString()}
                    </span>
                  </div>
                  <p className="text-sm">{reply.content}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

interface TaskDetailProps {
  task: Task;
}

export function TaskDetail({ task }: TaskDetailProps) {
  const { data: comments = [], isLoading: commentsLoading } = useComments(task.id);

  return (
    <div>
      <div className="flex items-start justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold">{task.title}</h1>
        <Badge className={`${stageColors[task.stage]} text-white`}>
          {stageLabels[task.stage]}
        </Badge>
      </div>

      <TaskSection title="Instructions">
        <div className="prose prose-sm dark:prose-invert max-w-none">
          {task.description || <p className="text-muted-foreground italic">No instructions yet</p>}
        </div>
      </TaskSection>

      <TaskSection title="Context">
        <div className="prose prose-sm dark:prose-invert max-w-none">
          {task.context || <p className="text-muted-foreground italic">No context added yet</p>}
        </div>
      </TaskSection>

      <TaskSection title="Updates" defaultOpen={false}>
        <p className="text-muted-foreground italic">Updates from Claude sessions will appear here</p>
      </TaskSection>

      <TaskSection title="Comments">
        {commentsLoading ? (
          <p className="text-muted-foreground">Loading comments...</p>
        ) : (
          <CommentThreadPlaceholder taskId={task.id} comments={comments} />
        )}
      </TaskSection>
    </div>
  );
}
