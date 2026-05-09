'use client';

import { useState } from 'react';
import { ChevronDown, ChevronRight, AlertTriangle, Clock } from 'lucide-react';
import { useAllSessions } from '@/hooks/use-sessions';
import { useTasks } from '@/hooks/use-tasks';
import type { Session } from '@/types/session';

import { statusColor } from '@/lib/chart-colors';

function shortId(id: string) {
  return id.slice(0, 8);
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleString([], {
      month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit', second: '2-digit',
    });
  } catch {
    return iso;
  }
}

function ageMinutes(iso: string): number {
  return (Date.now() - new Date(iso).getTime()) / 60_000;
}

interface Props {
  onSessionClick: (sessionId: string) => void;
  onTaskClick: (taskId: string) => void;
  taskId?: string;
}

type StatusFilter = 'all' | 'failed' | 'pending' | 'stuck';

export function SessionDiagnosticsPanel({ onSessionClick, onTaskClick, taskId }: Props) {
  const [open, setOpen] = useState(true);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const { data: sessions = [], isLoading } = useAllSessions(['failed', 'pending'], 200);
  const { data: tasks = [] } = useTasks();

  const taskMap = new Map(tasks.map((t) => [t.id, t.title]));

  const filtered = sessions.filter((s: Session) => {
    if (taskId && s.task_id !== taskId) return false;
    if (statusFilter === 'failed') return s.status === 'failed';
    if (statusFilter === 'pending') return s.status === 'pending';
    if (statusFilter === 'stuck') return s.status === 'pending' && ageMinutes(s.started_at) > 5;
    return true;
  });

  const failedCount = sessions.filter((s: Session) => s.status === 'failed').length;
  const stuckCount = sessions.filter((s: Session) => s.status === 'pending' && ageMinutes(s.started_at) > 5).length;

  return (
    <div className="rounded-xl border border-border bg-card overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border">
        <button
          onClick={() => setOpen((o) => !o)}
          className="flex items-center gap-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wide hover:text-foreground transition-colors shrink-0"
        >
          <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />
          Session Diagnostics
          {open ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>

        {/* Status counts */}
        <div className="flex gap-2 ml-2">
          {failedCount > 0 && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-red-500/10 text-red-500 font-medium">
              {failedCount} failed
            </span>
          )}
          {stuckCount > 0 && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-orange-500/10 text-orange-500 font-medium flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {stuckCount} stuck
            </span>
          )}
        </div>

        {/* Filter pills */}
        <div className="ml-auto flex gap-1">
          {(['all', 'failed', 'pending', 'stuck'] as StatusFilter[]).map((f) => (
            <button
              key={f}
              onClick={() => setStatusFilter(f)}
              className={`px-2 py-0.5 rounded text-xs font-medium transition-colors ${
                statusFilter === f
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground hover:bg-muted/70'
              }`}
            >
              {f === 'stuck' ? 'Stuck (>5m)' : f}
            </button>
          ))}
        </div>
      </div>

      {/* Body */}
      {open && (
        <div className="p-4">
          {isLoading ? (
            <div className="h-24 bg-muted rounded animate-shimmer" />
          ) : filtered.length === 0 ? (
            <div className="py-6 text-center text-sm text-muted-foreground">
              {statusFilter === 'all'
                ? 'No failed or pending sessions — everything looks healthy'
                : `No ${statusFilter} sessions`}
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-muted-foreground border-b border-border">
                    <th className="text-left pb-1.5 pr-3 font-medium">Status</th>
                    <th className="text-left pb-1.5 pr-3 font-medium">Task</th>
                    <th className="text-left pb-1.5 pr-3 font-medium">Session ID</th>
                    <th className="text-left pb-1.5 pr-3 font-medium">Started</th>
                    <th className="text-left pb-1.5 font-medium">Error / Age</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {filtered.map((s: Session) => {
                    const taskTitle = taskMap.get(s.task_id);
                    const isStuck = s.status === 'pending' && ageMinutes(s.started_at) > 5;
                    return (
                      <tr key={s.id} className="hover:bg-muted/50 transition-colors">
                        <td className="py-1.5 pr-3">
                          <span
                            className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium"
                            style={{ background: `${statusColor(s.status)}22`, color: statusColor(s.status) }}
                          >
                            <span className="w-1.5 h-1.5 rounded-full inline-block" style={{ background: statusColor(s.status) }} />
                            {isStuck ? 'stuck' : s.status}
                          </span>
                        </td>
                        <td className="py-1.5 pr-3">
                          <button
                            className="text-primary hover:underline truncate max-w-[140px] block text-left"
                            title={taskTitle ?? s.task_id}
                            onClick={() => onTaskClick(s.task_id)}
                          >
                            {taskTitle ?? shortId(s.task_id) + '…'}
                          </button>
                        </td>
                        <td className="py-1.5 pr-3">
                          <button
                            className="font-mono text-indigo-400 hover:underline"
                            title={s.id}
                            onClick={() => onSessionClick(s.id)}
                          >
                            {shortId(s.id)}…
                          </button>
                        </td>
                        <td className="py-1.5 pr-3 text-muted-foreground whitespace-nowrap">
                          {formatTime(s.started_at)}
                        </td>
                        <td className="py-1.5 max-w-xs">
                          {s.error_message ? (
                            <span className="text-destructive truncate block" title={s.error_message}>
                              {s.error_message}
                            </span>
                          ) : isStuck ? (
                            <span className="text-orange-500">
                              Pending for {Math.round(ageMinutes(s.started_at))}m — may be stuck
                            </span>
                          ) : (
                            <span className="text-muted-foreground">—</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
