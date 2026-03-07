# Task Detail Redesign + Session Error Display

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show session start errors inline on the task detail page, and redesign the page to be polished and mobile-friendly.

**Architecture:** All changes are frontend-only. `session-controls.tsx` captures API errors and surfaces them as an inline red alert. `task-detail.tsx` is rebuilt with a hero header, metadata chips, and Card-based sections. The page wrapper gets a sticky mobile header.

**Tech Stack:** Next.js, React, Tailwind, shadcn/ui (Badge, Button, Card, Alert), lucide-react, date-fns

---

### Task 1: Inline error display in SessionControls

**Files:**
- Modify: `frontend/src/components/sessions/session-controls.tsx`

The `ApiError` message is the raw response body from the server, e.g. `{"error":"Project path does not exist: /home/..."}`. Parse it to extract the human-readable message. Show it as a red alert below the buttons. Clear on next attempt.

**Implementation:**

```tsx
'use client';

import { useState } from 'react';
import { Play, Square, RotateCcw, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { apiClient, ApiError } from '@/lib/api-client';
import { useQueryClient } from '@tanstack/react-query';
import { logger } from '@/lib/logger';
import type { SessionStatus } from '@/types/session';

interface SessionControlsProps {
  taskId: string;
  sessionId?: string | null;
  status?: SessionStatus | null;
  hasClaudeComments?: boolean;
}

function parseApiError(err: unknown): string {
  if (err instanceof ApiError) {
    try {
      const parsed = JSON.parse(err.message);
      if (parsed?.error) return parsed.error;
    } catch {}
    return err.message;
  }
  return String(err);
}

export function SessionControls({ taskId, sessionId, status, hasClaudeComments = false }: SessionControlsProps) {
  const queryClient = useQueryClient();
  const [sessionError, setSessionError] = useState<string | null>(null);

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
    queryClient.invalidateQueries({ queryKey: ['sessions'] });
    queryClient.invalidateQueries({ queryKey: ['comments', taskId] });
  };

  const log = logger.withContext({ task_id: taskId, session_id: sessionId ?? undefined });

  const startSession = async () => {
    setSessionError(null);
    log.info('Starting new Claude session', { task_id: taskId });
    try {
      await apiClient(`/api/tasks/${taskId}/sessions`, { method: 'POST' });
      log.info('Session started successfully', { task_id: taskId });
      invalidate();
    } catch (err) {
      log.error('Failed to start session', { task_id: taskId, error: String(err) });
      setSessionError(parseApiError(err));
    }
  };

  const continueSession = async () => {
    setSessionError(null);
    log.info('Continuing Claude session', { task_id: taskId, session_id: sessionId });
    try {
      await apiClient(`/api/tasks/${taskId}/sessions/continue`, { method: 'POST' });
      log.info('Continue session enqueued', { task_id: taskId, session_id: sessionId });
      invalidate();
    } catch (err) {
      log.error('Failed to continue session', { task_id: taskId, session_id: sessionId, error: String(err) });
      setSessionError(parseApiError(err));
    }
  };

  const stopSession = async () => {
    if (!sessionId) return;
    setSessionError(null);
    log.info('Stopping session', { task_id: taskId, session_id: sessionId });
    try {
      await apiClient(`/api/sessions/${sessionId}/stop`, { method: 'POST' });
      log.info('Session stopped', { task_id: taskId, session_id: sessionId });
      invalidate();
    } catch (err) {
      log.error('Failed to stop session', { task_id: taskId, session_id: sessionId, error: String(err) });
      setSessionError(parseApiError(err));
    }
  };

  return (
    <div className="space-y-3">
      {status === 'pending' && (
        <Button disabled size="sm">
          <Play className="mr-2 h-4 w-4" />
          Starting...
        </Button>
      )}
      {status === 'running' && (
        <Button onClick={stopSession} variant="destructive" size="sm">
          <Square className="mr-2 h-4 w-4" />
          Stop Session
        </Button>
      )}
      {(status === 'completed' || status === 'failed' || status === 'stopped' || !status) && (
        <div className="flex flex-wrap gap-2">
          <Button onClick={startSession} size="sm">
            <Play className="mr-2 h-4 w-4" />
            Start Session
          </Button>
          {hasClaudeComments && (
            <Button onClick={continueSession} variant="outline" size="sm">
              <RotateCcw className="mr-2 h-4 w-4" />
              Continue Session
            </Button>
          )}
        </div>
      )}
      {sessionError && (
        <div className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
          <span>{sessionError}</span>
        </div>
      )}
    </div>
  );
}
```

### Task 2: Redesign task-detail.tsx

**Files:**
- Modify: `frontend/src/components/tasks/task-detail.tsx`

Key changes:
- Hero section: large title, stage badge, project path chip, timestamps row
- Card-based layout (shadcn Card) replacing bordered TaskSection boxes
- Session card shows inline error from SessionControls (already handled in Task 1)
- Updates & Comments remain collapsible
- All touch targets ≥ 44px, single column layout (mobile-first)

```tsx
'use client';

import { useState } from 'react';
import { formatDistanceToNow, format } from 'date-fns';
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
        {open ? <ChevronDown className="h-4 w-4 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
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
        <div className="prose prose-sm dark:prose-invert max-w-none text-sm leading-relaxed whitespace-pre-wrap pr-8">
          {value}
        </div>
      ) : (
        <p className="text-muted-foreground italic text-sm pr-8">{placeholder}</p>
      )}
      <button
        onClick={() => { setDraft(value ?? ''); setEditing(true); }}
        className="absolute top-0 right-0 opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-muted min-h-[36px] min-w-[36px] flex items-center justify-center"
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
      <div className="space-y-3">
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
        <div className="flex flex-wrap items-center gap-2 text-sm">
          <Badge variant="outline" className={cn('font-medium', stage.className)}>
            {stage.label}
          </Badge>
          <div className="flex items-center gap-1.5 text-muted-foreground text-xs bg-muted/50 rounded-md px-2 py-1">
            <FolderOpen className="h-3.5 w-3.5 shrink-0" />
            <span className="font-mono truncate max-w-[200px] sm:max-w-xs">{task.project_path}</span>
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
          <CardTitle className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">Instructions</CardTitle>
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
          <CardTitle className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">Context</CardTitle>
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
          <CardTitle className="text-sm font-semibold text-muted-foreground uppercase tracking-wide flex items-center gap-2">
            <Terminal className="h-4 w-4" />
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
              <p className="text-xs text-muted-foreground font-mono">
                Session: {task.session_id}
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
                  <div className="text-sm prose prose-sm dark:prose-invert max-w-none">
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>{update.content}</ReactMarkdown>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </CollapsibleCard>

      {/* Comments */}
      <CollapsibleCard title="Comments" defaultOpen={true}>
        {commentsLoading ? (
          <p className="text-muted-foreground text-sm">Loading comments...</p>
        ) : (
          <CommentThread taskId={task.id} comments={comments.filter((c) => c.author !== 'litellm')} />
        )}
      </CollapsibleCard>
    </div>
  );
}
```

### Task 3: Update page wrapper for mobile

**Files:**
- Modify: `frontend/src/app/tasks/[id]/page.tsx`

Sticky header on scroll, responsive padding.

```tsx
'use client';

import { useParams, useRouter } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Sidebar } from '@/components/layout/sidebar';
import { useTask, useDeleteTask } from '@/hooks/use-tasks';
import { TaskDetail } from '@/components/tasks/task-detail';
import { TaskDetailSkeleton } from '@/components/tasks/task-detail-skeleton';

export default function TaskDetailPage() {
  const params = useParams();
  const router = useRouter();
  const taskId = params.id as string;

  const { data: task, isLoading, error } = useTask(taskId);
  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();

  const handleDelete = () => {
    deleteTask(taskId, { onSuccess: () => router.push('/') });
  };

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 min-w-0">
        <header className="sticky top-0 z-10 border-b border-border bg-background/95 backdrop-blur-sm px-4 sm:px-6 py-3 flex items-center gap-3">
          <Button variant="ghost" size="sm" onClick={() => router.back()} className="-ml-1">
            <ArrowLeft className="mr-1.5 h-4 w-4" />
            <span className="hidden sm:inline">Back</span>
          </Button>
          {task && (
            <h2 className="text-sm font-medium text-muted-foreground truncate">{task.title}</h2>
          )}
        </header>
        <main className="max-w-3xl mx-auto px-4 sm:px-6 py-6">
          {isLoading ? (
            <TaskDetailSkeleton />
          ) : error || !task ? (
            <div className="flex items-center justify-center py-24">
              <p className="text-destructive">Task not found</p>
            </div>
          ) : (
            <TaskDetail task={task} onDelete={handleDelete} isDeleting={isDeleting} />
          )}
        </main>
      </div>
    </div>
  );
}
```

### Task 4: Verify visually

Open http://localhost:3002/tasks/f9e5beb7-c130-4634-8d0f-928512f4f9cc in the browser. Click "Start Session" — should see the red error banner with "Project path does not exist: /home/utility/Projects/particle_simulator". Check mobile layout at 375px width.
