'use client';

import { useEffect, useRef, useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Trash2, Pencil, Check, X, FolderOpen, Clock, ChevronDown, ChevronRight, Terminal } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { CommentThread } from './comment-thread';
import { ActivityTimeline } from './activity-timeline';
import { SessionControls } from '@/components/sessions/session-controls';
import { LiveOutputPanel } from '@/components/sessions/live-output-panel';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useComments } from '@/hooks/use-comments';
import { useSession } from '@/hooks/use-sessions';
import { useUpdateTask } from '@/hooks/use-tasks';
import { useWebSocket } from '@/contexts/websocket-context';
import { apiClient } from '@/lib/api-client';
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
  readOnly = false,
}: {
  label: string;
  value: string | null;
  placeholder: string;
  taskId: string;
  field: 'description' | 'context' | 'instructions';
  readOnly?: boolean;
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
        <div className="text-sm leading-relaxed pr-8 [&_p]:mb-2 [&_p:last-child]:mb-0 [&_ul]:list-disc [&_ul]:pl-4 [&_ul]:mb-2 [&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:mb-2 [&_li]:mb-1 [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-xs [&_code]:font-mono [&_pre]:bg-muted [&_pre]:p-3 [&_pre]:rounded [&_pre]:overflow-x-auto [&_pre]:mb-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_strong]:font-semibold [&_h1]:text-base [&_h1]:font-semibold [&_h1]:mb-2 [&_h2]:text-sm [&_h2]:font-semibold [&_h2]:mb-1 [&_h3]:text-sm [&_h3]:font-medium [&_h3]:mb-1">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{value}</ReactMarkdown>
        </div>
      ) : (
        <p className="text-muted-foreground italic text-sm pr-8">{placeholder}</p>
      )}
      {!readOnly && (
        <button
          onClick={() => { setDraft(value ?? ''); setEditing(true); }}
          className="absolute top-0 right-0 opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded hover:bg-muted"
          title={`Edit ${label}`}
        >
          <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
        </button>
      )}
    </div>
  );
}

// ─── Plan Checklist Viewer ────────────────────────────────────────────────────

interface ChecklistStats {
  total: number;
  completed: number;
}

function parseChecklist(content: string): ChecklistStats {
  const lines = content.split('\n');
  let total = 0;
  let completed = 0;
  for (const line of lines) {
    if (/^[\s]*-\s+\[[ xX]\]/.test(line)) {
      total++;
      if (/^[\s]*-\s+\[[xX]\]/.test(line)) completed++;
    }
  }
  return { total, completed };
}

function renderChecklistMarkdown(content: string): string {
  // Replace checkbox markdown syntax with unicode checkboxes for display
  return content
    .replace(/^([\s]*)-\s+\[[xX]\]/gm, '$1- ☑')
    .replace(/^([\s]*)-\s+\[ \]/gm, '$1- ☐');
}

function PlanProgress({ instructions }: { instructions: string }) {
  const stats = parseChecklist(instructions);
  if (stats.total === 0) return null;

  const pct = Math.round((stats.completed / stats.total) * 100);
  return (
    <div className="mb-4 space-y-1.5">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>Implementation Plan ({stats.completed} of {stats.total} tasks complete)</span>
        <span className="flex items-center gap-1">
          <span className="text-[10px] bg-purple-500/10 text-purple-600 dark:text-purple-400 px-1.5 py-0.5 rounded">AI-generated</span>
          <span>{pct}%</span>
        </span>
      </div>
      <div className="w-full h-2 bg-muted rounded-full overflow-hidden">
        <div
          className="h-full bg-emerald-500 transition-all duration-500"
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

// ─── Plan Banner ─────────────────────────────────────────────────────────────

function EnrichmentBanner({ taskId }: { taskId: string }) {
  const { subscribe } = useWebSocket();
  const [enriching, setEnriching] = useState(false);

  useEffect(() => {
    const unsubStart = subscribe('enrichment_started', (data: unknown) => {
      const msg = data as { task_id?: string };
      if (msg.task_id === taskId) setEnriching(true);
    });
    const unsubDone = subscribe('enrichment_completed', (data: unknown) => {
      const msg = data as { task_id?: string };
      if (msg.task_id === taskId) setEnriching(false);
    });
    return () => { unsubStart(); unsubDone(); };
  }, [taskId, subscribe]);

  if (!enriching) return null;

  return (
    <div className="mb-3 flex items-center gap-2 rounded-md border border-indigo-300 bg-indigo-50 dark:bg-indigo-900/20 dark:border-indigo-700 px-3 py-2 text-sm text-indigo-800 dark:text-indigo-200">
      <span className="h-2 w-2 rounded-full bg-indigo-500 animate-pulse shrink-0" />
      <span>Enriching task description with AI — instructions will appear shortly...</span>
    </div>
  );
}

function PlanBanner({ taskId, sessionId }: { taskId: string; sessionId: string | null }) {
  const { subscribe } = useWebSocket();
  const [show, setShow] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!sessionId) return;
    return subscribe('plan_created', (data: unknown) => {
      const msg = data as { session_id?: string; task_id?: string };
      if (msg.task_id !== taskId && msg.session_id !== sessionId) return;
      setShow(true);
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => setShow(false), 10_000);
    });
  }, [sessionId, taskId, subscribe]);

  useEffect(() => () => { if (timerRef.current) clearTimeout(timerRef.current); }, []);

  if (!show) return null;

  return (
    <div className="mb-3 flex items-center justify-between gap-2 rounded-md border border-amber-300 bg-amber-50 dark:bg-amber-900/20 dark:border-amber-700 px-3 py-2 text-sm text-amber-800 dark:text-amber-200">
      <span>✅ Implementation plan written by Claude. Ready to start working.</span>
      <button
        onClick={() => setShow(false)}
        className="ml-2 shrink-0 text-amber-600 hover:text-amber-900 dark:text-amber-400"
        aria-label="Dismiss"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

// ─── Queue Position Badge ─────────────────────────────────────────────────────

interface QueueStatusResponse {
  active_count: number;
  queued: Array<{ task_id: string; [key: string]: unknown }>;
}

function QueueBadge({ taskId, sessionStatus }: { taskId: string; sessionStatus: string | null }) {
  const [inQueue, setInQueue] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const isActive = sessionStatus === 'pending' || sessionStatus === 'running';

  useEffect(() => {
    if (!isActive) {
      setInQueue(false);
      return;
    }

    const fetchQueue = async () => {
      try {
        const data = await apiClient<QueueStatusResponse>('/api/sessions');
        const queued = Array.isArray(data.queued) ? data.queued : [];
        setInQueue(queued.some((q) => q.task_id === taskId));
      } catch {
        // Graceful degradation — ignore errors
      }
    };

    fetchQueue();
    intervalRef.current = setInterval(fetchQueue, 5_000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [taskId, isActive]);

  if (!inQueue) return null;

  return (
    <span className="inline-flex items-center gap-1 text-[10px] font-medium px-2 py-0.5 rounded-full border border-amber-400 bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300 dark:border-amber-600">
      <span className="h-1.5 w-1.5 rounded-full bg-amber-500 animate-pulse" />
      In Queue
    </span>
  );
}

// ─── Main TaskDetail ──────────────────────────────────────────────────────────

export function TaskDetail({ task, onDelete = () => {}, isDeleting }: TaskDetailProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { data: comments = [], isLoading: commentsLoading } = useComments(task.id);
  const { data: session } = useSession(task.session_id);
  const sessionStatus = session?.status ?? null;
  // Show "Continue Session" if Claude actually ran (has a claude_session_id), regardless of comments
  const canResume = !!session?.claude_session_id;
  const stage = stageConfig[task.stage];

  // Rendered checklist markdown
  const instructionsForDisplay = task.instructions
    ? renderChecklistMarkdown(task.instructions)
    : null;

  const hasCheckboxes = task.instructions ? parseChecklist(task.instructions).total > 0 : false;

  // Session is active (pending/running) with no plan yet
  const isSessionActive = sessionStatus === 'running' || sessionStatus === 'pending';
  const showPlanWriting = !task.instructions && task.stage === 'planning' && isSessionActive;

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
          <QueueBadge taskId={task.id} sessionStatus={sessionStatus} />
          <div className="flex items-center gap-1.5 text-muted-foreground text-xs bg-muted/60 rounded-md px-2.5 py-1">
            <FolderOpen className="h-3.5 w-3.5 shrink-0" />
            <span className="font-mono truncate max-w-[180px] sm:max-w-xs">{task.project_path}</span>
          </div>
          <div className="flex items-center gap-1.5 text-muted-foreground text-xs">
            <Clock className="h-3.5 w-3.5 shrink-0" />
            <UpdatedTime updatedAt={task.updated_at} />
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

      {/* Description (user-owned, never modified by LiteLLM) */}
      <Card>
        <CardHeader className="pb-2 px-5 pt-5">
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Description</CardTitle>
        </CardHeader>
        <CardContent className="px-5 pb-5">
          <InlineEditField
            label="Description"
            value={task.description}
            placeholder="No description yet — click to add"
            taskId={task.id}
            field="description"
          />
        </CardContent>
      </Card>

      {/* Instructions / Plan Viewer */}
      {(task.instructions || true) && (
        <Card>
          <CardHeader className="pb-2 px-5 pt-5">
            <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
              Instructions
              {task.instructions && (
                <span className="text-[10px] font-normal normal-case tracking-normal bg-purple-500/10 text-purple-600 dark:text-purple-400 px-1.5 py-0.5 rounded">AI-enriched</span>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="px-5 pb-5">
            {/* Enrichment in-progress banner */}
            <EnrichmentBanner taskId={task.id} />
            {/* Plan Created banner (subscribes to WS event) */}
            <PlanBanner taskId={task.id} sessionId={task.session_id ?? null} />

            {/* Plan writing indicator */}
            {showPlanWriting && (
              <div className="mb-3 flex items-center gap-2 text-sm text-muted-foreground">
                <span className="h-2 w-2 rounded-full bg-blue-500 animate-pulse shrink-0" />
                <span className="italic">Claude is writing the implementation plan...</span>
              </div>
            )}

            {/* Progress bar for checkbox plans */}
            {task.instructions && hasCheckboxes && (
              <PlanProgress instructions={task.instructions} />
            )}

            <InlineEditField
              label="Instructions"
              value={instructionsForDisplay}
              placeholder="No enriched instructions yet — will be generated by LiteLLM when session starts"
              taskId={task.id}
              field="instructions"
            />
          </CardContent>
        </Card>
      )}

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
              <div className="text-xs text-muted-foreground [&_p]:mb-2 [&_p:last-child]:mb-0 [&_ul]:list-disc [&_ul]:pl-4 [&_ul]:mb-2 [&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:mb-2 [&_li]:mb-1 [&_strong]:font-semibold [&_h1]:text-xs [&_h1]:font-semibold [&_h2]:text-xs [&_h2]:font-semibold [&_h3]:text-xs [&_h3]:font-medium">
                <ReactMarkdown remarkPlugins={[remarkGfm]}>{task.compressed_context}</ReactMarkdown>
              </div>
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
            hasClaudeComments={canResume}
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

      {/* Activity Timeline (replaces Updates) */}
      <CollapsibleCard title="Activity" defaultOpen={true}>
        {commentsLoading ? (
          <div className="h-16 animate-pulse bg-muted rounded" />
        ) : (
          <ActivityTimeline task={task} sessionId={task.session_id ?? null} />
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

// Client-only relative time to avoid SSR mismatch
function UpdatedTime({ updatedAt }: { updatedAt: string }) {
  const [display, setDisplay] = useState<string | null>(null);
  useEffect(() => {
    setDisplay(formatDistanceToNow(new Date(updatedAt), { addSuffix: true }));
  }, [updatedAt]);
  if (!display) return null;
  return <span>Updated {display}</span>;
}
