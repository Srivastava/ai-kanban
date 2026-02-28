'use client';

import { Badge } from '@/components/ui/badge';
import { TaskSection } from './task-section';
import { CommentThread } from './comment-thread';
import { useComments } from '@/hooks/use-comments';
import type { Task, Stage } from '@/types/task';

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
          <CommentThread taskId={task.id} comments={comments} />
        )}
      </TaskSection>
    </div>
  );
}
