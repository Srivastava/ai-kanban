'use client';

import { useMemo, useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import type { LogEntry } from '@/types/log';

interface Props {
  logs: LogEntry[];
  onTaskClick: (taskId: string) => void;
  onSessionClick: (sessionId: string) => void;
  activeTaskId?: string;
  activeSessionId?: string;
}

function BarRow({
  id,
  count,
  max,
  color,
  isActive,
  onClick,
}: {
  id: string;
  count: number;
  max: number;
  color: string;
  isActive: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left group p-1.5 rounded-md transition-colors ${
        isActive ? 'bg-primary/10' : 'hover:bg-muted/40'
      }`}
    >
      <div className="flex items-center justify-between mb-1">
        <span className={`text-xs font-mono ${color} group-hover:underline`}>
          {id.slice(0, 8)}…
        </span>
        <span className="text-xs text-muted-foreground tabular-nums">{count} logs</span>
      </div>
      <div className="h-1.5 rounded-full bg-muted overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${isActive ? 'opacity-100' : 'opacity-70 group-hover:opacity-100'}`}
          style={{
            width: `${Math.max(4, (count / max) * 100)}%`,
            background: color.includes('primary') ? 'var(--primary)' : 'var(--stage-review)',
          }}
        />
      </div>
    </button>
  );
}

export function LogContextBreakdown({ logs, onTaskClick, onSessionClick, activeTaskId, activeSessionId }: Props) {
  const [open, setOpen] = useState(true);

  const { topTasks, topSessions, totalWithContext } = useMemo(() => {
    const taskCounts = new Map<string, number>();
    const sessionCounts = new Map<string, number>();
    let totalWithContext = 0;

    for (const log of logs) {
      if (log.task_id) {
        taskCounts.set(log.task_id, (taskCounts.get(log.task_id) ?? 0) + 1);
        totalWithContext++;
      }
      if (log.session_id) {
        sessionCounts.set(log.session_id, (sessionCounts.get(log.session_id) ?? 0) + 1);
      }
    }

    const topTasks = Array.from(taskCounts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6)
      .map(([id, count]) => ({ id, count }));

    const topSessions = Array.from(sessionCounts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6)
      .map(([id, count]) => ({ id, count }));

    return { topTasks, topSessions, totalWithContext };
  }, [logs]);

  if (topTasks.length === 0 && topSessions.length === 0) return null;

  const maxTask = topTasks[0]?.count ?? 1;
  const maxSession = topSessions[0]?.count ?? 1;
  const pctWithContext = logs.length > 0 ? Math.round((totalWithContext / logs.length) * 100) : 0;

  return (
    <div className="rounded-xl border border-border bg-card overflow-hidden">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-4 py-2.5 text-xs font-semibold text-muted-foreground uppercase tracking-wide hover:bg-muted/40 transition-colors"
      >
        <span className="w-2 h-2 rounded-full bg-stage-review/60" />
        Log Context Breakdown
        <span className="text-muted-foreground/60 font-normal normal-case ml-1">
          {pctWithContext}% of logs have task/session context
        </span>
        {open ? <ChevronDown className="h-3.5 w-3.5 ml-auto" /> : <ChevronRight className="h-3.5 w-3.5 ml-auto" />}
      </button>

      {open && (
        <div className="border-t border-border grid grid-cols-2 divide-x divide-border">
          {/* Tasks */}
          <div className="p-4">
            <p className="text-xs font-medium text-muted-foreground mb-2">
              Top Tasks
              <span className="text-muted-foreground/60 ml-1 font-normal">(click to filter)</span>
            </p>
            {topTasks.length === 0 ? (
              <p className="text-xs text-muted-foreground italic">No task-tagged logs</p>
            ) : (
              <div className="space-y-0.5">
                {topTasks.map(({ id, count }) => (
                  <BarRow
                    key={id}
                    id={id}
                    count={count}
                    max={maxTask}
                    color="text-primary"
                    isActive={activeTaskId === id}
                    onClick={() => onTaskClick(id)}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Sessions */}
          <div className="p-4">
            <p className="text-xs font-medium text-muted-foreground mb-2">
              Top Sessions (internal ID)
              <span className="text-muted-foreground/60 ml-1 font-normal">(click to filter)</span>
            </p>
            {topSessions.length === 0 ? (
              <p className="text-xs text-muted-foreground italic">No session-tagged logs</p>
            ) : (
              <div className="space-y-0.5">
                {topSessions.map(({ id, count }) => (
                  <BarRow
                    key={id}
                    id={id}
                    count={count}
                    max={maxSession}
                    color="text-stage-review-text"
                    isActive={activeSessionId === id}
                    onClick={() => onSessionClick(id)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
