'use client';

import { useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Trash2, Pencil, Check, X, FolderOpen, Clock, ChevronDown, ChevronRight, Terminal } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { CommentThread } from './comment-thread';
import { SessionControls } from '@/components/sessions/session-controls';
import { LiveOutputPanel } from '@/components/sessions/live-output-panel';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useComments } from '@/hooks/use-comments';
import { useSession } from '@/hooks/use-sessions';
import { useUpdateTask } from '@/hooks/use-tasks';
import type { Task, Stage } from '@/types/task';
import { cn } from '@/lib/utils';

const stageConfig: Record<Stage, { label: string; className: string }> = {
  backlog:     { label: 'Backlog',     className: 'bg-slate-500/15 text-slate-600 dark:text-slate-400 border-slate-500/20' },
  planning:    { label: 'Planning',    className: 'bg-blue-500/15 text-blue-600 dark:text-blue-400 border-blue-500/20' },
  ready:       { label: 'Ready',       className: 'bg-amber-500/15 text-amber-600 dark:text-amber-400 border-amber-500/20' },
  in_progress: { label: 'In Progress', className: 'bg-orange-500/15 text-orange-600 dark:text-orange-400 border-orange-500/20' },
  review:      { label: 'Review',      className: 'bg-purple-500/15 text-purple-600 dark:text-purple-400 border-purple-500/20' },
  done:        { label: 'Done',        className: 'bg-green-500/15 text-green-600 dark:text-green-400 border-green-500/20' },
};

interface TaskDetailProps {
  task: Task;
  onDelete?: () => void;
  isDeleting?: boolean;
}

function CollapsibleCard({
  title,
  defaultOpen = true,
  children,
  icon,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
  icon?: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <Card>
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 w-full px-5 py-4 text-left font-semibold text-sm hover:bg-muted/40 transition-colors rounded-t-lg"
      >
        {icon && <span className="text-muted-foreground">{icon}</span>}
        <span className="flex-1">{title}</span>
        {open
          ? <ChevronDown className="h-4 w-4 text-muted-foreground" />
          : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
      </button>
      {open && <CardContent className="pt-0 px-5 pb-5">{children}</CardContent>}
    </Card>
  );
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
    updateTask({ id: taskId, data: { [field]: draft || null } }, { onSuccess: () => setEditing(false) });
  };
  const handleCancel = () => { setDraft(value ?? ''); setEditing(false); };

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
            <Check className="h-3.5 w-3.5 mr-1" /> Save
          </Button>
          <Button size="sm" variant="ghost" onClick={handleCancel} disabled={isPending}>
            <X className="h-3.5 w-3.5 mr-1" /> Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="group relative">
      {value ? (
        <p className="text-sm leading-relaxed whitespace-pre-wrap pr-8">{value}</p>
      ) : (
        <p className="text-muted-foreground italic text-sm pr-8">{placeholder}</p>
      )}
      <button
        onClick={() => { setDraft(value ?? ''); setEditing(true); }}
        className="absolute top-0 right-0 opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded hover:bg-muted"
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
  const hasClaudeComments = comments.some((c) => c.author === 'claude' || c.author === 'litellm');
  const stage = stageConfig[task.stage];

  const updates = comments
    .flatMap((c) => [c, ...c.replies])
    .filter((c) => c.author === 'litellm')
    .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

  return (
    <div className="space-y-4">
      {/* Hero */}
      <div className="space-y-3 pb-2">
        <div className="flex items-start justify-between gap-3">
          <h1 className="text-2xl font-bold leading-tight">{task.title}</h1>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Delete task"
            className="text-muted-foreground hover:text-destructive hover:bg-destructive/10 shrink-0"
            onClick={() => setConfirmOpen(true)}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>

        {/* Metadata chips */}
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="outline" className={cn('font-medium text-xs', stage.className)}>
            {stage.label}
          </Badge>
          <div className="flex items-center gap-1.5 text-muted-foreground text-xs bg-muted/60 rounded-md px-2.5 py-1">
            <FolderOpen className="h-3.5 w-3.5 shrink-0" />
            <span className="font-mono truncate max-w-[180px] sm:max-w-xs">{task.project_path}</span>
          </div>
          <div className="flex items-center gap-1.5 text-muted-foreground text-xs">
            <Clock className="h-3.5 w-3.5 shrink-0" />
            <span>Updated {formatDistanceToNow(new Date(task.updated_at), { addSuffix: true })}</span>
          </div>
        </div>
      </div>

      <ConfirmDeleteDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={task.title}
        onConfirm={onDelete}
        isDeleting={isDeleting}
      />

      {/* Instructions */}
      <Card>
        <CardHeader className="pb-2 px-5 pt-5">
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Instructions</CardTitle>
        </CardHeader>
        <CardContent className="px-5 pb-5">
          <InlineEditField
            label="Instructions"
            value={task.description}
            placeholder="No instructions yet — click to add"
            taskId={task.id}
            field="description"
          />
        </CardContent>
      </Card>

      {/* Context */}
      <Card>
        <CardHeader className="pb-2 px-5 pt-5">
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Context</CardTitle>
        </CardHeader>
        <CardContent className="px-5 pb-5 space-y-3">
          <InlineEditField
            label="Context"
            value={task.context}
            placeholder="No context added yet — click to add"
            taskId={task.id}
            field="context"
          />
          {task.compressed_context && (
            <div className="rounded-lg border border-border bg-muted/30 p-3 space-y-1">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Compressed Context</p>
              <p className="text-xs text-muted-foreground whitespace-pre-wrap">{task.compressed_context}</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Session */}
      <Card>
        <CardHeader className="pb-2 px-5 pt-5">
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
            <Terminal className="h-3.5 w-3.5" />
            Session
          </CardTitle>
        </CardHeader>
        <CardContent className="px-5 pb-5 space-y-3">
          <SessionControls
            taskId={task.id}
            sessionId={task.session_id ?? undefined}
            status={sessionStatus}
            hasClaudeComments={hasClaudeComments}
          />
          {task.session_id && (
            <>
              <p className="text-xs text-muted-foreground font-mono break-all">
                ID: {task.session_id}
              </p>
              <LiveOutputPanel
                sessionId={task.session_id}
                status={sessionStatus}
                initialClaudeSessionId={session?.claude_session_id}
              />
            </>
          )}
        </CardContent>
      </Card>

      {/* Updates */}
      <CollapsibleCard title="Updates" defaultOpen={updates.length > 0}>
        {commentsLoading ? (
          <div className="h-16 animate-pulse bg-muted rounded" />
        ) : updates.length === 0 ? (
          <p className="text-muted-foreground italic text-sm">No updates yet — Claude will post updates here as it works</p>
        ) : (
          <div className="space-y-4">
            {updates.map((update) => (
              <div key={update.id} className="flex gap-3">
                <div className="w-1.5 h-1.5 rounded-full bg-purple-500 mt-2 shrink-0" />
                <div className="min-w-0">
                  <p className="text-xs text-muted-foreground mb-1">
                    {formatDistanceToNow(new Date(update.created_at), { addSuffix: true })}
                  </p>
                  <div className="text-sm [&_p]:mb-2 [&_p:last-child]:mb-0 [&_ul]:list-disc [&_ul]:pl-4 [&_ul]:mb-2 [&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:mb-2 [&_li]:mb-1 [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-xs [&_code]:font-mono [&_pre]:bg-muted [&_pre]:p-3 [&_pre]:rounded [&_pre]:overflow-x-auto [&_pre]:mb-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_strong]:font-semibold">
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>{update.content}</ReactMarkdown>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </CollapsibleCard>

      {/* Comments */}
      <CollapsibleCard title="Comments">
        {commentsLoading ? (
          <p className="text-muted-foreground text-sm">Loading comments...</p>
        ) : (
          <CommentThread taskId={task.id} comments={comments.filter((c) => c.author !== 'litellm')} />
        )}
      </CollapsibleCard>
    </div>
  );
}
