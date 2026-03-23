'use client';

import {
  LineChart, Line, AreaChart, Area,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend,
} from 'recharts';
import { useDevActivity, useLocHistory, useTaskSessions, useTokensByTask } from '@/hooks/use-analytics';
import { useState } from 'react';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return `${n}`;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
  color: 'hsl(var(--card-foreground))',
};

interface Props { taskId?: string | null }

export function DevActivityCharts({ taskId: externalTaskId }: Props) {
  const [internalTaskId, setInternalTaskId] = useState<string | null>(null);
  const selectedTaskId = externalTaskId ?? internalTaskId ?? null;
  const { data: tasks = [] } = useTokensByTask();
  const { data = [], isLoading } = useDevActivity(selectedTaskId);
  const { data: locHistory = [] } = useLocHistory(selectedTaskId);
  const { data: sessions = [] } = useTaskSessions(selectedTaskId);

  const row = data[0] ?? null;

  // LOC growth line chart data
  const locData = locHistory.map((entry) => ({
    label: `#${entry.session_index}`,
    loc: entry.project_loc,
  }));

  // Token stacked area chart data — sessions ordered chronologically.
  // Note: SessionDetail does not include cache_creation/cache_read tokens,
  // so we show only input + output (two-layer stack).
  const sessionTokenData = [...sessions]
    .reverse() // sessions come newest-first, reverse for timeline
    .map((s, i) => ({
      label: `#${i + 1}`,
      input: s.input_tokens,
      output: s.output_tokens,
    }));

  const skeleton = <div className="h-40 animate-pulse bg-muted rounded" />;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      {/* Task selector — hidden when external taskId provided */}
      {!externalTaskId && (
        <select
          className="rounded-md border border-border bg-background px-3 py-1.5 text-sm max-w-xs"
          value={internalTaskId ?? ''}
          onChange={(e) => setInternalTaskId(e.target.value || null)}
        >
          <option value="">Select a task…</option>
          {tasks.map((t) => (
            <option key={t.task_id} value={t.task_id}>{t.task_title}</option>
          ))}
        </select>
      )}

      {!selectedTaskId ? (
        <div className="h-40 flex items-center justify-center rounded-lg border border-dashed border-border">
          <p className="text-muted-foreground text-sm">Select a task to view dev activity</p>
        </div>
      ) : isLoading ? (
        skeleton
      ) : !row ? (
        <div className="h-40 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No data for this task</p>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Summary row */}
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Sessions</p>
              <p className="font-semibold">{row.session_count}</p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Lines written (net)</p>
              <p className="font-semibold">
                {row.lines_added > 0
                  ? <span className="text-green-600">+{Math.round(row.lines_added).toLocaleString()}</span>
                  : '—'}
              </p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Tokens (in / cached / out)</p>
              <p className="font-semibold text-sm">
                <span className="text-blue-500">{formatTokens(row.input_tokens ?? 0)}</span>
                {' / '}
                <span className="text-amber-500">{formatTokens((row.cache_creation_tokens ?? 0) + (row.cache_read_tokens ?? 0))}</span>
                {' / '}
                <span className="text-violet-500">{formatTokens(row.output_tokens ?? 0)}</span>
              </p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Cost (OTel)</p>
              <p className="font-semibold">{row.cost_usd > 0 ? `$${row.cost_usd.toFixed(4)}` : '—'}</p>
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            {/* LOC growth line chart */}
            <div className="rounded-xl border border-border bg-card p-5 space-y-2">
              <h3 className="font-semibold text-sm">Project LOC Over Sessions</h3>
              <p className="text-xs text-muted-foreground">How the codebase grew session by session</p>
              {locData.length < 2 ? (
                <div className="h-40 flex items-center justify-center">
                  <p className="text-xs text-muted-foreground italic">
                    {locData.length === 0 ? 'No LOC snapshots yet' : 'Only one session — need more to show trend'}
                  </p>
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={160}>
                  <LineChart data={locData} margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <YAxis
                      tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : `${v}`}
                      tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                    />
                    <Tooltip
                      formatter={(value) => [typeof value === 'number' ? value.toLocaleString() : value, 'Lines of code']}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Line
                      type="monotone"
                      dataKey="loc"
                      stroke="#22c55e"
                      strokeWidth={2}
                      dot={{ fill: '#22c55e', r: 3 }}
                      activeDot={{ r: 5 }}
                    />
                  </LineChart>
                </ResponsiveContainer>
              )}
            </div>

            {/* Token stacked area chart */}
            <div className="rounded-xl border border-border bg-card p-5 space-y-2">
              <h3 className="font-semibold text-sm">Token Usage Per Session</h3>
              <p className="text-xs text-muted-foreground">Input and output tokens per session</p>
              {sessionTokenData.length < 2 ? (
                <div className="h-40 flex items-center justify-center">
                  <p className="text-xs text-muted-foreground italic">
                    {sessionTokenData.length === 0 ? 'No session data' : 'Only one session — need more to show trend'}
                  </p>
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={160}>
                  <AreaChart data={sessionTokenData} margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <Tooltip
                      formatter={(value, name) => {
                        const labels: Record<string, string> = { input: 'Input', output: 'Output' };
                        return [formatTokens(Number(value)), labels[String(name)] ?? String(name)];
                      }}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Legend formatter={(v) => ({ input: 'Input', output: 'Output' }[v as 'input' | 'output'] ?? v)} iconType="circle" />
                    <Area type="monotone" dataKey="input"  stackId="1" stroke="#6366f1" fill="#6366f1" fillOpacity={0.6} />
                    <Area type="monotone" dataKey="output" stackId="1" stroke="#a855f7" fill="#a855f7" fillOpacity={0.6} />
                  </AreaChart>
                </ResponsiveContainer>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
