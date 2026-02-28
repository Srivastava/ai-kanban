# Task Delete + Log Widgets Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add delete buttons (with confirmation) to all 3 task views, and enhance the logs page with a stats bar, recent errors panel, log rate sparkline, timezone-aware timestamps, and a copyable task ID column.

**Architecture:** Task delete reuses the existing `useDeleteTask` hook and backend `DELETE /api/tasks/:id` route — only UI wiring is needed. A shared `ConfirmDeleteDialog` component is extracted to avoid repeating the confirm pattern. Log widgets are pure client-side components that derive data from the already-loaded `logs` array passed down from the page; no new API endpoints needed.

**Tech Stack:** Next.js 16 (App Router), React, Tailwind CSS, Radix UI Dialog, Recharts (already installed), `Intl.DateTimeFormat` for timezone (no new deps), `useDeleteTask` / `useRouter` from existing hooks.

---

## Task 1: Shared ConfirmDeleteDialog component

**Files:**
- Create: `frontend/src/components/tasks/confirm-delete-dialog.tsx`

**Step 1: Create the component**

```tsx
'use client';

import {
  Dialog, DialogContent, DialogHeader, DialogTitle,
  DialogDescription, DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  onConfirm: () => void;
  isDeleting?: boolean;
}

export function ConfirmDeleteDialog({ open, onOpenChange, title, onConfirm, isDeleting }: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>Delete task?</DialogTitle>
          <DialogDescription>
            <span className="font-medium text-foreground">&ldquo;{title}&rdquo;</span> will be
            permanently deleted. This cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isDeleting}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm} disabled={isDeleting}>
            {isDeleting ? 'Deleting…' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

**Step 2: Verify it renders (visual check only — no automated test needed for a dialog wrapper)**

---

## Task 2: Delete button on task detail page

**Files:**
- Modify: `frontend/src/components/tasks/task-detail.tsx`
- Modify: `frontend/src/app/tasks/[id]/page.tsx`

**Step 1: Add delete to `task-detail.tsx`**

Add `onDelete` prop and a red Delete button in the header area:

```tsx
'use client';

import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Trash2 } from 'lucide-react';
import { TaskSection } from './task-section';
import { CommentThread } from './comment-thread';
import { SessionControls } from '@/components/sessions/session-controls';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useComments } from '@/hooks/use-comments';
import type { Task, Stage } from '@/types/task';

// keep stageColors and stageLabels as-is

interface TaskDetailProps {
  task: Task;
  onDelete: () => void;
  isDeleting?: boolean;
}

export function TaskDetail({ task, onDelete, isDeleting }: TaskDetailProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { data: comments = [], isLoading: commentsLoading } = useComments(task.id);

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

      {/* rest of TaskSection blocks unchanged */}
    </div>
  );
}
```

**Step 2: Wire delete in `tasks/[id]/page.tsx`**

```tsx
'use client';

import { useParams, useRouter } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';
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
    deleteTask(taskId, {
      onSuccess: () => router.push('/'),
    });
  };

  // loading/error guards unchanged

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b border-border px-6 py-4">
        <Button variant="ghost" onClick={() => router.back()}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back
        </Button>
      </header>
      <main className="max-w-4xl mx-auto p-6">
        <TaskDetail task={task} onDelete={handleDelete} isDeleting={isDeleting} />
      </main>
    </div>
  );
}
```

**Step 3: Verify in browser — open a task detail, click trash, confirm, task disappears and redirects to `/`**

---

## Task 3: Delete button on task-card.tsx (list view)

**Files:**
- Modify: `frontend/src/components/tasks/task-card.tsx`

**Step 1: Replace file content**

The card is wrapped in a `<Link>`. Render the trash button outside the link using absolute positioning, and stop propagation so clicking the icon doesn't navigate.

```tsx
'use client';

import { useState } from 'react';
import Link from 'next/link';
import { Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useDeleteTask } from '@/hooks/use-tasks';
import type { Task, Stage } from '@/types/task';

// keep stageColors and stageLabels as-is

interface TaskCardProps {
  task: Task;
}

export function TaskCard({ task }: TaskCardProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();
  const createdDate = new Date(task.created_at).toLocaleDateString();

  return (
    <div className="relative group">
      <Link href={`/tasks/${task.id}`}>
        <Card className="hover:shadow-md transition-shadow cursor-pointer">
          <CardHeader className="pb-2">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-base font-medium line-clamp-2 pr-6">
                {task.title}
              </CardTitle>
              <Badge className={`${stageColors[task.stage]} text-white shrink-0`}>
                {stageLabels[task.stage]}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            {task.description && (
              <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
                {task.description}
              </p>
            )}
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>Priority: {task.priority}</span>
              <span>{createdDate}</span>
            </div>
          </CardContent>
        </Card>
      </Link>

      {/* Trash button floats over top-right, outside the Link */}
      <Button
        variant="ghost"
        size="icon"
        className="absolute top-2 right-2 h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
        onClick={(e) => { e.preventDefault(); setConfirmOpen(true); }}
      >
        <Trash2 className="h-3.5 w-3.5" />
      </Button>

      <ConfirmDeleteDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={task.title}
        onConfirm={() => deleteTask(task.id)}
        isDeleting={isDeleting}
      />
    </div>
  );
}
```

**Step 2: Verify — hover a task card, trash icon appears; click it, confirm dialog opens; confirm, card disappears**

---

## Task 4: Delete button on kanban-card.tsx

**Files:**
- Modify: `frontend/src/components/kanban/kanban-card.tsx`

**Step 1: Replace file content**

Same pattern as task-card: drag handle stays on the outer div, trash icon floats over it.

```tsx
'use client';

import { useState } from 'react';
import Link from 'next/link';
import { Trash2 } from 'lucide-react';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ConfirmDeleteDialog } from '@/components/tasks/confirm-delete-dialog';
import { useDeleteTask } from '@/hooks/use-tasks';
import type { Task, Stage } from '@/types/task';

// keep stageColors as-is

interface KanbanCardProps {
  task: Task;
}

export function KanbanCard({ task }: KanbanCardProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();

  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: task.id });

  const style = { transform: CSS.Transform.toString(transform), transition };

  return (
    <div ref={setNodeRef} style={style} className={`relative group ${isDragging ? 'opacity-50' : ''}`}>
      {/* Drag handle only on the card, not on trash button */}
      <div {...attributes} {...listeners}>
        <Link href={`/tasks/${task.id}`}>
          <Card className="cursor-pointer hover:shadow-md transition-shadow">
            <CardHeader className="p-3 pb-1">
              <div className="flex items-start justify-between gap-2">
                <CardTitle className="text-sm font-medium line-clamp-2 pr-5">
                  {task.title}
                </CardTitle>
                <Badge className={`${stageColors[task.stage]} text-white text-[10px] px-1.5`}>
                  {task.priority}
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="p-3 pt-0">
              {task.description && (
                <p className="text-xs text-muted-foreground line-clamp-2">
                  {task.description}
                </p>
              )}
            </CardContent>
          </Card>
        </Link>
      </div>

      <Button
        variant="ghost"
        size="icon"
        className="absolute top-1.5 right-1.5 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
        onClick={(e) => { e.preventDefault(); e.stopPropagation(); setConfirmOpen(true); }}
      >
        <Trash2 className="h-3 w-3" />
      </Button>

      <ConfirmDeleteDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={task.title}
        onConfirm={() => deleteTask(task.id)}
        isDeleting={isDeleting}
      />
    </div>
  );
}
```

**Step 2: Verify — hover kanban card, trash appears; confirm dialog works; card removed from board**

---

## Task 5: Timezone-aware timestamps + task ID copy in log-table.tsx

**Files:**
- Modify: `frontend/src/components/logs/log-table.tsx`

**Step 1: Replace the timestamp formatter**

Replace `import { format } from 'date-fns';` with a local formatter using `Intl.DateTimeFormat`:

```ts
// At top of file, after imports
const TZ = 'America/Los_Angeles';

function formatLogTime(iso: string): string {
  const d = new Date(iso);
  return new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
    hour12: false,
  }).format(d);
}

function formatFullTime(iso: string): string {
  const d = new Date(iso);
  return new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
    hour12: false,
    timeZoneName: 'short',
  }).format(d);
}
```

**Step 2: Update time cell and task ID cell**

- Time cell: `{formatLogTime(log.timestamp)}` and `title={formatFullTime(log.timestamp)}`
- Task ID cell: add clipboard copy on click

```tsx
<td className="px-3 py-1.5 font-mono text-muted-foreground whitespace-nowrap">
  {log.task_id ? (
    <button
      className="hover:text-foreground transition-colors cursor-pointer"
      title={`Click to copy: ${log.task_id}`}
      onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(log.task_id!); }}
    >
      {log.task_id.slice(0, 8)}…
    </button>
  ) : '—'}
</td>
```

- In the expanded detail row, also update timestamp display: `{formatFullTime(log.timestamp)}`

**Step 3: Remove `date-fns` import** (no longer needed in this file)

---

## Task 6: Log stats bar component

**Files:**
- Create: `frontend/src/components/logs/log-stats-bar.tsx`

**Step 1: Create component**

```tsx
'use client';

import { cn } from '@/lib/utils';
import type { LogEntry, LogLevel } from '@/types/log';

const LEVEL_CONFIG: Record<LogLevel, { label: string; bg: string; text: string; activeBg: string }> = {
  DEBUG: { label: 'DEBUG', bg: 'bg-muted/50', text: 'text-muted-foreground', activeBg: 'bg-muted' },
  INFO:  { label: 'INFO',  bg: 'bg-blue-500/10',  text: 'text-blue-400',  activeBg: 'bg-blue-500/25' },
  WARN:  { label: 'WARN',  bg: 'bg-amber-500/10', text: 'text-amber-400', activeBg: 'bg-amber-500/25' },
  ERROR: { label: 'ERROR', bg: 'bg-red-500/10',   text: 'text-red-400',   activeBg: 'bg-red-500/25' },
};

const LEVELS: LogLevel[] = ['ERROR', 'WARN', 'INFO', 'DEBUG'];

interface Props {
  logs: LogEntry[];
  activeLevel?: LogLevel;
  onLevelClick: (level: LogLevel | undefined) => void;
}

export function LogStatsBar({ logs, activeLevel, onLevelClick }: Props) {
  const counts = logs.reduce<Record<LogLevel, number>>(
    (acc, log) => { acc[log.level as LogLevel] = (acc[log.level as LogLevel] ?? 0) + 1; return acc; },
    { DEBUG: 0, INFO: 0, WARN: 0, ERROR: 0 }
  );

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {LEVELS.map((level) => {
        const cfg = LEVEL_CONFIG[level];
        const isActive = activeLevel === level;
        return (
          <button
            key={level}
            onClick={() => onLevelClick(isActive ? undefined : level)}
            className={cn(
              'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border',
              isActive
                ? `${cfg.activeBg} ${cfg.text} border-current/30`
                : `${cfg.bg} ${cfg.text} border-transparent hover:border-current/20`
            )}
          >
            <span className="font-mono">{cfg.label}</span>
            <span className="font-bold tabular-nums">{counts[level]}</span>
          </button>
        );
      })}
      <span className="text-xs text-muted-foreground ml-1">
        {logs.length} total loaded
      </span>
    </div>
  );
}
```

---

## Task 7: Recent errors panel component

**Files:**
- Create: `frontend/src/components/logs/recent-errors-panel.tsx`

**Step 1: Create component**

```tsx
'use client';

import { useState } from 'react';
import { ChevronDown, ChevronRight, AlertCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { LogEntry } from '@/types/log';

const TZ = 'America/Los_Angeles';
function fmtTime(iso: string) {
  return new Intl.DateTimeFormat('en-US', {
    timeZone: TZ, hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
  }).format(new Date(iso));
}

interface Props {
  logs: LogEntry[];          // full unfiltered log list
  onJumpToLog?: (id: number) => void;
}

export function RecentErrorsPanel({ logs, onJumpToLog }: Props) {
  const [open, setOpen] = useState(true);

  const errors = logs
    .filter((l) => l.level === 'ERROR')
    .slice(0, 5);

  if (errors.length === 0) return null;

  return (
    <div className="rounded-xl border border-red-500/30 bg-red-500/5 overflow-hidden">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-4 py-2.5 text-sm font-medium text-red-400 hover:bg-red-500/10 transition-colors"
      >
        <AlertCircle className="h-4 w-4 shrink-0" />
        <span>Recent Errors ({errors.length})</span>
        {open ? <ChevronDown className="h-3.5 w-3.5 ml-auto" /> : <ChevronRight className="h-3.5 w-3.5 ml-auto" />}
      </button>

      {open && (
        <div className="divide-y divide-red-500/10">
          {errors.map((log) => (
            <button
              key={log.id}
              onClick={() => onJumpToLog?.(log.id)}
              className="w-full flex items-start gap-3 px-4 py-2 text-xs hover:bg-red-500/10 transition-colors text-left"
            >
              <span className="font-mono text-muted-foreground shrink-0 mt-0.5">{fmtTime(log.timestamp)}</span>
              <span className="font-mono text-purple-400 shrink-0 mt-0.5">{log.source}</span>
              <span className="text-foreground/90 truncate">{log.message}</span>
              {log.task_id && (
                <span className="font-mono text-muted-foreground shrink-0 ml-auto">
                  {log.task_id.slice(0, 8)}
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

---

## Task 8: Log rate sparkline component

**Files:**
- Create: `frontend/src/components/logs/log-rate-chart.tsx`

**Step 1: Create component**

Groups loaded logs into 1-minute buckets, renders a small AreaChart colored by error presence.

```tsx
'use client';

import { useMemo } from 'react';
import { AreaChart, Area, XAxis, Tooltip, ResponsiveContainer } from 'recharts';
import type { LogEntry } from '@/types/log';

const TZ = 'America/Los_Angeles';

function bucketByMinute(logs: LogEntry[]) {
  const buckets = new Map<string, { total: number; errors: number }>();

  for (const log of logs) {
    const d = new Date(log.timestamp);
    const label = new Intl.DateTimeFormat('en-US', {
      timeZone: TZ, hour: '2-digit', minute: '2-digit', hour12: false,
    }).format(d);

    const existing = buckets.get(label) ?? { total: 0, errors: 0 };
    existing.total += 1;
    if (log.level === 'ERROR') existing.errors += 1;
    buckets.set(label, existing);
  }

  return Array.from(buckets.entries())
    .map(([label, v]) => ({ label, ...v }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

interface Props {
  logs: LogEntry[];
}

export function LogRateChart({ logs }: Props) {
  const data = useMemo(() => bucketByMinute(logs), [logs]);

  if (data.length < 2) return null;

  const hasErrors = data.some((d) => d.errors > 0);

  return (
    <div className="rounded-xl border border-border bg-card/50 px-4 pt-3 pb-2">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-muted-foreground">Log rate (per minute)</span>
        <span className="text-xs text-muted-foreground">{data.length} min window</span>
      </div>
      <ResponsiveContainer width="100%" height={64}>
        <AreaChart data={data} margin={{ top: 2, right: 2, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id="logGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={hasErrors ? '#ef4444' : '#6366f1'} stopOpacity={0.3} />
              <stop offset="95%" stopColor={hasErrors ? '#ef4444' : '#6366f1'} stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis dataKey="label" hide />
          <Tooltip
            contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '6px', fontSize: '11px' }}
            formatter={(value, name) => [value, name === 'total' ? 'logs' : 'errors']}
          />
          <Area type="monotone" dataKey="total" stroke={hasErrors ? '#ef4444' : '#6366f1'}
            fill="url(#logGrad)" strokeWidth={1.5} dot={false} />
          {hasErrors && (
            <Area type="monotone" dataKey="errors" stroke="#ef4444"
              fill="none" strokeWidth={1} strokeDasharray="3 3" dot={false} />
          )}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
```

---

## Task 9: Wire all log widgets into logs/page.tsx

**Files:**
- Modify: `frontend/src/app/logs/page.tsx`

**Step 1: Update imports and add widget row**

Add the three new components above the table, and wire `LogStatsBar` `onLevelClick` to `handleLevelFilterChange`. Pass full (unfiltered) `logs` to `RecentErrorsPanel` and `LogRateChart`.

```tsx
import { LogStatsBar } from '@/components/logs/log-stats-bar';
import { RecentErrorsPanel } from '@/components/logs/recent-errors-panel';
import { LogRateChart } from '@/components/logs/log-rate-chart';
```

Replace the `<main>` section:

```tsx
<main className="flex-1 p-6 space-y-4">
  {/* Stats bar */}
  <LogStatsBar
    logs={logs}
    activeLevel={levelFilter}
    onLevelClick={handleLevelFilterChange}
  />

  {/* Log rate sparkline */}
  <LogRateChart logs={logs} />

  {/* Recent errors */}
  <RecentErrorsPanel logs={logs} />

  {/* Table */}
  {isLoading ? (
    <div className="space-y-2">
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} className="h-9 animate-pulse bg-muted rounded" />
      ))}
    </div>
  ) : (
    <LogTable logs={logs} filter={clientFilter} />
  )}
</main>
```

**Step 2: Verify all widgets appear and stats bar level click updates the filter**

---

## Task 10: Commit

```bash
git add \
  frontend/src/components/tasks/confirm-delete-dialog.tsx \
  frontend/src/components/tasks/task-detail.tsx \
  frontend/src/components/tasks/task-card.tsx \
  frontend/src/components/kanban/kanban-card.tsx \
  frontend/src/app/tasks/[id]/page.tsx \
  frontend/src/components/logs/log-table.tsx \
  frontend/src/components/logs/log-stats-bar.tsx \
  frontend/src/components/logs/recent-errors-panel.tsx \
  frontend/src/components/logs/log-rate-chart.tsx \
  frontend/src/app/logs/page.tsx
git commit -m "feat: task delete with confirmation + log debug widgets"
```
