'use client';

import { useState } from 'react';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend } from 'recharts';
import { useDevActivity } from '@/hooks/use-analytics';
import { useTokensByTask } from '@/hooks/use-analytics';

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
};

interface Props { taskId?: string | null }

export function DevActivityCharts({ taskId: externalTaskId }: Props) {
  const [internalTaskId, setInternalTaskId] = useState<string | null>(null);
  const selectedTaskId = externalTaskId ?? internalTaskId;
  const { data: tasks = [] } = useTokensByTask();
  const { data = [], isLoading } = useDevActivity(selectedTaskId);

  const row = data[0] ?? null;

  const skeleton = <div className="h-32 animate-pulse bg-muted rounded" />;

  const empty = (
    <div className="h-32 flex items-center justify-center">
      <p className="text-muted-foreground text-sm">No data for this task</p>
    </div>
  );

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
            <option key={t.task_id} value={t.task_id}>
              {t.task_title}
            </option>
          ))}
        </select>
      )}

      {!selectedTaskId ? (
        <div className="h-32 flex items-center justify-center rounded-lg border border-dashed border-border">
          <p className="text-muted-foreground text-sm">Select a task to view dev activity</p>
        </div>
      ) : isLoading ? (
        skeleton
      ) : !row ? (
        empty
      ) : (
        <div className="space-y-4">
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

          {/* Charts */}
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            {/* Lines of Code */}
            <div className="rounded-xl border border-border bg-card p-5 space-y-2">
              <h3 className="font-semibold text-sm">Lines Written (net growth)</h3>
              <p className="text-xs text-muted-foreground">Project LOC growth across sessions — current minus baseline</p>
              {row.lines_added === 0 ? (
                <div className="h-32 flex items-center justify-center">
                  <p className="text-xs text-muted-foreground italic">No session LOC data yet</p>
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={160}>
                  <BarChart
                    data={[{ name: row.task_title, loc: Math.round(row.lines_added) }]}
                    margin={{ top: 5, right: 10, left: 10, bottom: 5 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                    <XAxis dataKey="name" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <YAxis tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : `${v}`} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <Tooltip
                      formatter={(value) => [typeof value === 'number' ? value.toLocaleString() : String(value), 'Lines written (net)']}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Bar dataKey="loc" fill="#22c55e" name="loc" radius={[3, 3, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              )}
            </div>

            {/* Token Usage */}
            <div className="rounded-xl border border-border bg-card p-5 space-y-2">
              <h3 className="font-semibold text-sm">Token Usage</h3>
              <p className="text-xs text-muted-foreground">Input + output tokens across all sessions</p>
              {row.input_tokens === 0 && row.output_tokens === 0 ? (
                <div className="h-32 flex items-center justify-center">
                  <p className="text-xs text-muted-foreground italic">No token data for this task</p>
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={160}>
                  <BarChart
                    data={[{ name: row.task_title, input: row.input_tokens, output: row.output_tokens }]}
                    margin={{ top: 5, right: 10, left: 10, bottom: 5 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                    <XAxis dataKey="name" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                    <Tooltip
                      formatter={(value, name) => [formatTokens(Number(value)), name === 'input' ? 'Input tokens' : 'Output tokens']}
                      contentStyle={TOOLTIP_STYLE}
                    />
                    <Legend formatter={(v) => (v === 'input' ? 'Input' : 'Output')} iconType="circle" />
                    <Bar dataKey="input" fill="#6366f1" name="input" radius={[3, 3, 0, 0]} />
                    <Bar dataKey="output" fill="#a855f7" name="output" radius={[3, 3, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
