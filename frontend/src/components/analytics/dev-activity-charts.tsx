'use client';

import {
  LineChart, Line, AreaChart, Area,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend,
} from 'recharts';
import { useDevActivity, useLocHistory, useTaskSessions, useTokensByTask } from '@/hooks/use-analytics';
import { useState } from 'react';
import { formatTokens } from '@/lib/format';
import { TOKEN_COLORS } from '@/lib/chart-colors';

const TOOLTIP_STYLE = {
  background: 'var(--card)',
  border: '1px solid var(--border)',
  borderRadius: '8px',
  fontSize: '12px',
  color: 'var(--card-foreground)',
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

  const locData = locHistory.map((entry) => ({
    label: `#${entry.session_index}`,
    loc: entry.project_loc,
  }));

  const sessionTokenData = [...sessions]
    .reverse()
    .map((s, i) => ({
      label: `#${i + 1}`,
      input: s.input_tokens,
      output: s.output_tokens,
    }));

  const skeleton = <div className="h-40 bg-muted rounded animate-shimmer" />;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      {/* Task selector — hidden when external taskId provided */}
      {!externalTaskId && (
        <div className="flex flex-col gap-1">
          <label htmlFor="dev-activity-task-select" className="sr-only">Select task</label>
          <select
            id="dev-activity-task-select"
            className="rounded-md border border-border bg-background px-3 py-1.5 text-sm max-w-xs"
            value={internalTaskId ?? ''}
            onChange={(e) => setInternalTaskId(e.target.value || null)}
          >
            <option value="">Select a task…</option>
            {tasks.map((t) => (
              <option key={t.task_id} value={t.task_id}>{t.task_title}</option>
            ))}
          </select>
        </div>
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
                  ? <span className="text-stage-done-text">+{Math.round(row.lines_added).toLocaleString()}</span>
                  : '—'}
              </p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Tokens (in / cached / out)</p>
              <p className="font-semibold text-sm">
                <span className="text-stage-planning-text">{formatTokens(row.input_tokens ?? 0)}</span>
                {' / '}
                <span className="text-stage-ready-text">{formatTokens((row.cache_creation_tokens ?? 0) + (row.cache_read_tokens ?? 0))}</span>
                {' / '}
                <span className="text-stage-review-text">{formatTokens(row.output_tokens ?? 0)}</span>
              </p>
            </div>
            <div className="rounded-lg bg-muted/50 px-3 py-2">
              <p className="text-xs text-muted-foreground">Cost (OTel)</p>
              <p className="font-semibold">{row.cost_usd > 0 ? `$${row.cost_usd.toFixed(4)}` : '—'}</p>
            </div>
          </div>

          {/* Charts — flat sections, no nested cards */}
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
            {/* LOC growth line chart */}
            <div className="space-y-2">
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
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }} />
                    <YAxis
                      tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : `${v}`}
                      tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
                    />
                    <Tooltip
                      formatter={(value) => [typeof value === 'number' ? value.toLocaleString() : value, 'Lines of code']}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Line
                      type="monotone"
                      dataKey="loc"
                      stroke={TOKEN_COLORS.output}
                      strokeWidth={2}
                      dot={{ fill: TOKEN_COLORS.output, r: 3 }}
                      activeDot={{ r: 5 }}
                    />
                  </LineChart>
                </ResponsiveContainer>
              )}
            </div>

            {/* Token stacked area chart */}
            <div className="space-y-2">
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
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }} />
                    <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }} />
                    <Tooltip
                      formatter={(value, name) => {
                        const labels: Record<string, string> = { input: 'Input', output: 'Output' };
                        return [formatTokens(Number(value)), labels[String(name)] ?? String(name)];
                      }}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Legend formatter={(v) => ({ input: 'Input', output: 'Output' }[v as 'input' | 'output'] ?? v)} iconType="circle" />
                    <Area type="monotone" dataKey="input"  stackId="1" stroke={TOKEN_COLORS.input}  fill={TOKEN_COLORS.input}  fillOpacity={0.6} />
                    <Area type="monotone" dataKey="output" stackId="1" stroke={TOKEN_COLORS.output} fill={TOKEN_COLORS.output} fillOpacity={0.6} />
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
