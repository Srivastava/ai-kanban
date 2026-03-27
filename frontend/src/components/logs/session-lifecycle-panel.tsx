'use client';

import { useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell, CartesianGrid } from 'recharts';
import { useTaskSessions, useTokensByTask } from '@/hooks/use-analytics';
import type { SessionDetail } from '@/types/analytics';

import { statusColor } from '@/lib/chart-colors';
import { formatTokens } from '@/lib/format';

function formatDuration(secs: number | null): string {
  if (secs == null || secs < 0) return '—';
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  if (m < 60) return `${m}m ${s}s`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

function shortId(id: string) {
  return id.slice(0, 8);
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return iso;
  }
}

interface Props {
  taskId: string | null;
  activeSessionId?: string;
  activeClaudeSessionId?: string;
  onSessionClick: (sessionId: string) => void;
  onClaudeSessionClick: (claudeSessionId: string) => void;
  onTaskSelect?: (taskId: string) => void;
}

const TOOLTIP_STYLE = {
  background: 'var(--card)',
  border: '1px solid var(--border)',
  borderRadius: '8px',
  fontSize: '12px',
};

export function SessionLifecyclePanel({
  taskId,
  activeSessionId,
  activeClaudeSessionId,
  onSessionClick,
  onClaudeSessionClick,
  onTaskSelect,
}: Props) {
  const [open, setOpen] = useState(true);
  const { data: tasks = [] } = useTokensByTask();
  const { data: sessions = [], isLoading } = useTaskSessions(taskId);

  const selectedTask = tasks.find((t) => t.task_id === taskId);

  const barData = sessions
    .map((s, i) => ({
      label: `S${sessions.length - i}`,
      total_tokens: s.total_tokens,
      input_tokens: s.input_tokens,
      output_tokens: s.output_tokens,
      status: s.status,
      id: s.id,
      claude_session_id: s.claude_session_id,
    }))
    .reverse();

  return (
    <div className="rounded-xl border border-border bg-card overflow-hidden">
      {/* Header with collapse + task picker */}
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border">
        <button
          onClick={() => setOpen((o) => !o)}
          className="flex items-center gap-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wide hover:text-foreground transition-colors shrink-0"
        >
          <span className="w-2 h-2 rounded-full bg-stage-planning/60" />
          Session Lifecycle
          {open ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>
        <select
          className="ml-auto max-w-xs rounded-md border border-border bg-background px-2 py-1 text-xs"
          value={taskId ?? ''}
          onChange={(e) => onTaskSelect?.(e.target.value || '')}
        >
          <option value="">Select a task…</option>
          {tasks.map((t) => (
            <option key={t.task_id} value={t.task_id}>
              {t.task_title} — {t.total_tokens.toLocaleString()} tokens
            </option>
          ))}
        </select>
      </div>

      {/* Body */}
      {open && (
        <>
          {!taskId ? (
            <div className="px-4 py-8 text-center text-sm text-muted-foreground">
              Select a task above to view its session lifecycle
            </div>
          ) : isLoading ? (
            <div className="p-4 space-y-2">
              <div className="h-32 bg-muted rounded animate-shimmer" />
            </div>
          ) : sessions.length === 0 ? (
            <div className="px-4 py-8 text-center text-sm text-muted-foreground">
              No sessions found for{selectedTask ? ` "${selectedTask.task_title}"` : ' this task'}
            </div>
          ) : (
            <div className="p-4 space-y-4">
              <p className="text-xs text-muted-foreground">
                {sessions.length} session{sessions.length !== 1 ? 's' : ''}
                {selectedTask ? ` for "${selectedTask.task_title}"` : ''}
              </p>

              {/* Token bar chart */}
              <div>
                <p className="text-xs text-muted-foreground mb-1">Tokens per session</p>
                <ResponsiveContainer width="100%" height={120}>
                  <BarChart data={barData} margin={{ top: 4, right: 8, left: 0, bottom: 4 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }} />
                    <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }} width={36} />
                    <Tooltip
                      contentStyle={TOOLTIP_STYLE}
                      content={({ active, payload }) => {
                        if (!active || !payload?.length) return null;
                        const d = payload[0].payload as (typeof barData)[0];
                        return (
                          <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1">
                            <p className="font-medium" style={{ color: statusColor(d.status) }}>{d.status}</p>
                            <p className="text-muted-foreground font-mono">{shortId(d.id)}...</p>
                            <p>{d.input_tokens.toLocaleString()} in / {d.output_tokens.toLocaleString()} out</p>
                            <p className="font-medium">{d.total_tokens.toLocaleString()} total</p>
                          </div>
                        );
                      }}
                    />
                    <Bar dataKey="total_tokens" radius={[2, 2, 0, 0]}>
                      {barData.map((d, i) => (
                        <Cell key={i} fill={statusColor(d.status)} opacity={0.8} />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </div>

              {/* Session table */}
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-muted-foreground border-b border-border">
                      <th className="text-left pb-1.5 pr-3 font-medium">Status</th>
                      <th className="text-left pb-1.5 pr-3 font-medium">Internal ID</th>
                      <th className="text-left pb-1.5 pr-3 font-medium">Claude Session ID</th>
                      <th className="text-left pb-1.5 pr-3 font-medium">Started</th>
                      <th className="text-right pb-1.5 pr-3 font-medium">Duration</th>
                      <th className="text-right pb-1.5 font-medium">Tokens</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {sessions.map((s) => {
                      const isActiveInternal = activeSessionId === s.id;
                      const isActiveClaude = activeClaudeSessionId && s.claude_session_id === activeClaudeSessionId;
                      const highlight = isActiveInternal || isActiveClaude;
                      return (
                        <tr
                          key={s.id}
                          className={`transition-colors ${highlight ? 'bg-primary/10' : 'hover:bg-muted/50'}`}
                        >
                          <td className="py-1.5 pr-3">
                            <span
                              className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium"
                              style={{ background: `${statusColor(s.status)}22`, color: statusColor(s.status) }}
                            >
                              <span className="w-1.5 h-1.5 rounded-full inline-block" style={{ background: statusColor(s.status) }} />
                              {s.status}
                            </span>
                          </td>
                          <td className="py-1.5 pr-3">
                            <button
                              className="font-mono text-primary hover:underline"
                              onClick={() => onSessionClick(s.id)}
                              title={s.id}
                            >
                              {shortId(s.id)}…
                            </button>
                          </td>
                          <td className="py-1.5 pr-3">
                            {s.claude_session_id ? (
                              <button
                                className="font-mono text-violet-500 hover:underline"
                                onClick={() => onClaudeSessionClick(s.claude_session_id!)}
                                title={s.claude_session_id}
                              >
                                {shortId(s.claude_session_id)}…
                              </button>
                            ) : (
                              <span className="text-muted-foreground">—</span>
                            )}
                          </td>
                          <td className="py-1.5 pr-3 text-muted-foreground">{formatTime(s.started_at)}</td>
                          <td className="py-1.5 pr-3 text-right text-muted-foreground">{formatDuration(s.duration_secs)}</td>
                          <td className="py-1.5 text-right">
                            <span className="font-medium">{formatTokens(s.total_tokens)}</span>
                            {s.total_tokens > 0 && (
                              <span className="text-muted-foreground ml-1">
                                ({formatTokens(s.input_tokens)}↑ {formatTokens(s.output_tokens)}↓)
                              </span>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>

              {/* Error messages if any */}
              {sessions.some((s) => s.error_message) && (
                <div className="space-y-1">
                  <p className="text-xs font-medium text-destructive">Session errors</p>
                  {sessions
                    .filter((s) => s.error_message)
                    .map((s) => (
                      <div key={s.id} className="text-xs bg-destructive/10 border border-destructive/20 rounded p-2">
                        <span className="font-mono text-muted-foreground mr-2">{shortId(s.id)}…</span>
                        <span className="text-destructive">{s.error_message}</span>
                      </div>
                    ))}
                </div>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}
