'use client';

import { useState } from 'react';
import { BarChart, Bar, Cell, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useTokensByTask, useTaskSessions } from '@/hooks/use-analytics';
import { formatDistanceToNow } from 'date-fns';
import { SessionToolBreakdown } from './session-tool-breakdown';

const STATUS_COLORS: Record<string, string> = {
  completed: '#22c55e',
  failed:    '#ef4444',
  stopped:   '#f97316',
  running:   '#6366f1',
};

function statusColor(status: string): string {
  return STATUS_COLORS[status] ?? '#94a3b8';
}

function formatTokens(v: number) {
  return v >= 1000 ? `${(v / 1000).toFixed(0)}K` : String(v);
}

function formatDuration(secs: number | null): string {
  if (!secs) return '—';
  if (secs < 60) return `${secs}s`;
  return `${Math.floor(secs / 60)}m ${secs % 60}s`;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
  color: 'hsl(var(--card-foreground))',
};

interface Props { taskId?: string | null }

export function SessionTimelineChart({ taskId: externalTaskId }: Props) {
  const { data: tasks = [] } = useTokensByTask();
  const [internalTaskId, setInternalTaskId] = useState<string | null>(null);
  const [expandedSessionId, setExpandedSessionId] = useState<string | null>(null);
  const selectedTaskId = externalTaskId ?? internalTaskId;
  const { data: sessions = [], isLoading } = useTaskSessions(selectedTaskId);

  // Latest 20 sessions, reversed so the chart reads oldest → newest (left → right)
  const chartSessions = [...sessions].slice(0, 20).reverse();

  const barData = chartSessions.map((s, i) => ({
    label: `#${sessions.length - (chartSessions.length - 1 - i)}`,
    session_id: s.id,
    short_id: s.id.slice(0, 8),
    status: s.status,
    total_tokens: s.total_tokens,
    input_tokens: s.input_tokens,
    output_tokens: s.output_tokens,
    duration_secs: s.duration_secs,
    started_at: s.started_at,
    color: statusColor(s.status),
  }));

  const totalTokens = sessions.reduce((sum, s) => sum + s.total_tokens, 0);
  const completedCount = sessions.filter(s => s.status === 'completed').length;
  const failedCount = sessions.filter(s => s.status === 'failed').length;
  const stoppedCount = sessions.filter(s => s.status === 'stopped').length;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center gap-4 flex-wrap">
        <h3 className="font-semibold">Session Token Timeline</h3>
        {!externalTaskId && (
          <select
            className="flex-1 max-w-xs rounded-md border border-border bg-background px-3 py-1.5 text-sm"
            value={internalTaskId ?? ''}
            onChange={(e) => setInternalTaskId(e.target.value || null)}
          >
            <option value="">Select a task...</option>
            {tasks.map((t) => (
              <option key={t.task_id} value={t.task_id}>
                {t.task_title} — {t.total_tokens.toLocaleString()} tokens
              </option>
            ))}
          </select>
        )}
      </div>

      {!selectedTaskId ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">
            {externalTaskId ? 'Loading sessions…' : 'Select a task to view its session history'}
          </p>
        </div>
      ) : isLoading ? (
        <div className="h-64 animate-pulse bg-muted rounded" />
      ) : sessions.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No sessions found for this task</p>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Summary stats */}
          <div className="grid grid-cols-4 gap-3 text-sm">
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Total sessions</p>
              <p className="font-semibold">{sessions.length}</p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Total tokens</p>
              <p className="font-semibold">{formatTokens(totalTokens)}</p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Completed</p>
              <p className="font-semibold text-green-500">{completedCount}</p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Failed / Stopped</p>
              <p className="font-semibold text-red-400">{failedCount} / {stoppedCount}</p>
            </div>
          </div>

          {/* Status legend */}
          <div className="flex gap-4 text-xs text-muted-foreground items-center">
            {Object.entries(STATUS_COLORS).map(([status, color]) => (
              <span key={status} className="flex items-center gap-1.5">
                <span className="w-2.5 h-2.5 rounded-sm inline-block" style={{ background: color }} />
                {status}
              </span>
            ))}
            {sessions.length > 20 && (
              <span className="ml-auto italic">Showing latest 20 of {sessions.length}</span>
            )}
          </div>

          {/* Bar chart: total tokens per session, colored by status */}
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={barData} margin={{ top: 5, right: 10, left: 10, bottom: 20 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis
                dataKey="label"
                tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                label={{ value: 'Session (oldest → newest)', position: 'insideBottom', offset: -10, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }}
              />
              <YAxis
                tickFormatter={formatTokens}
                tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                label={{ value: 'Tokens', angle: -90, position: 'insideLeft', offset: 15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }}
              />
              <Tooltip
                content={({ active, payload }) => {
                  if (!active || !payload?.length) return null;
                  const d = payload[0].payload;
                  return (
                    <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1 shadow-sm">
                      <p className="font-medium capitalize" style={{ color: d.color }}>
                        {d.status} · {d.short_id}
                      </p>
                      <p className="text-muted-foreground">
                        {formatDistanceToNow(new Date(d.started_at), { addSuffix: true })}
                      </p>
                      <p>{d.input_tokens.toLocaleString()} in / {d.output_tokens.toLocaleString()} out</p>
                      <p className="font-medium">Total: {d.total_tokens.toLocaleString()} tokens</p>
                      <p className="text-muted-foreground">Duration: {formatDuration(d.duration_secs)}</p>
                    </div>
                  );
                }}
                contentStyle={TOOLTIP_STYLE}
              />
              <Bar
                dataKey="total_tokens"
                radius={[3, 3, 0, 0]}
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                onClick={(data: any) => {
                  setExpandedSessionId((prev: string | null) =>
                    prev === data.session_id ? null : data.session_id
                  );
                }}
                style={{ cursor: 'pointer' }}
              >
                {barData.map((entry) => (
                  <Cell key={entry.session_id} fill={entry.color}
                    opacity={expandedSessionId && expandedSessionId !== entry.session_id ? 0.4 : 1} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
          {expandedSessionId && (
            <div className="transition-all duration-200">
              <SessionToolBreakdown
                sessionId={expandedSessionId}
                sessionTotalTokens={
                  sessions.find(s => s.id === expandedSessionId)?.total_tokens ?? 0
                }
                onClose={() => setExpandedSessionId(null)}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
