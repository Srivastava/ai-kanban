'use client';

import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Trash2, Pencil, Check, X } from 'lucide-react';
import { TaskSection } from './task-section';
import { CommentThread } from './comment-thread';
import { SessionControls } from '@/components/sessions/session-controls';
import { LiveOutputPanel } from '@/components/sessions/live-output-panel';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useComments } from '@/hooks/use-comments';
import { useSession } from '@/hooks/use-sessions';
import { useUpdateTask } from '@/hooks/use-tasks';
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
  onDelete?: () => void;
  isDeleting?: boolean;
}

function InlineEditField({
  label,
  value,
  placeholder,
  taskId,
  field,
}: {
  label: string;
  value: string | null;
  placeholder: string;
  taskId: string;
  field: 'description' | 'context';
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value ?? '');
  const { mutate: updateTask, isPending } = useUpdateTask();

  const handleSave = () => {
    updateTask(
      { id: taskId, data: { [field]: draft || null } },
      { onSuccess: () => setEditing(false) }
    );
  };

  const handleCancel = () => {
    setDraft(value ?? '');
    setEditing(false);
  };

  if (editing) {
    return (
      <div className="space-y-2">
        <textarea
          className="w-full min-h-[120px] rounded-md border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring resize-y"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder={placeholder}
          autoFocus
        />
        <div className="flex gap-2">
          <Button size="sm" onClick={handleSave} disabled={isPending}>
            <Check className="h-3.5 w-3.5 mr-1" />
            Save
          </Button>
          <Button size="sm" variant="ghost" onClick={handleCancel} disabled={isPending}>
            <X className="h-3.5 w-3.5 mr-1" />
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="group relative">
      <div className="prose prose-sm dark:prose-invert max-w-none">
        {value ? (
          <p className="whitespace-pre-wrap">{value}</p>
        ) : (
          <p className="text-muted-foreground italic">{placeholder}</p>
        )}
      </div>
      <button
        onClick={() => { setDraft(value ?? ''); setEditing(true); }}
        className="absolute top-0 right-0 opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-muted"
        title={`Edit ${label}`}
      >
        <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
      </button>
    </div>
  );
}

export function TaskDetail({ task, onDelete = () => {}, isDeleting }: TaskDetailProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { data: comments = [], isLoading: commentsLoading } = useComments(task.id);
  const { data: session } = useSession(task.session_id);
  const sessionStatus = session?.status ?? null;
  const hasClaudeComments = comments.some((c) => c.author === 'claude');

  return (
    <div>
      <div className="flex items-start justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold">{task.title}</h1>
        <div className="flex items-center gap-2 shrink-0">
          <Badge className={`${stageColors[task.stage]} text-white`}>
            {stageLabels[task.stage]}
          </Badge>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Delete task"
            className="text-destructive hover:bg-destructive/10"
            onClick={() => setConfirmOpen(true)}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <ConfirmDeleteDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={task.title}
        onConfirm={onDelete}
        isDeleting={isDeleting}
      />

      <TaskSection title="Instructions">
        <InlineEditField
          label="Instructions"
          value={task.description}
          placeholder="No instructions yet — click to add"
          taskId={task.id}
          field="description"
        />
      </TaskSection>

      <TaskSection title="Context">
        <InlineEditField
          label="Context"
          value={task.context}
          placeholder="No context added yet — click to add"
          taskId={task.id}
          field="context"
        />
      </TaskSection>

      <TaskSection title="Session">
        <div className="space-y-3">
          <SessionControls
            taskId={task.id}
            sessionId={task.session_id ?? undefined}
            status={sessionStatus}
            hasClaudeComments={hasClaudeComments}
          />
          {task.session_id && (
            <>
              <p className="text-xs text-muted-foreground font-mono">
                Internal ID: {task.session_id}
              </p>
              <LiveOutputPanel
                sessionId={task.session_id}
                status={sessionStatus}
                initialClaudeSessionId={session?.claude_session_id}
              />
            </>
          )}
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
