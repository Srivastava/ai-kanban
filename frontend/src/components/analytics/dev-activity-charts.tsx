'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend } from 'recharts';
import { useDevActivity } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return n.toString();
}

function truncate(s: string, n = 14): string {
  return s.length > n ? s.slice(0, n) + '\u2026' : s;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

export function DevActivityCharts() {
  const { data = [], isLoading } = useDevActivity();

  const chartData = data.map((row) => ({
    label: truncate(row.task_title),
    fullTitle: row.task_title,
    sessions: row.session_count,
    added: Math.round(row.lines_added),
    deleted: Math.round(row.lines_deleted),
    hasLines: row.lines_added > 0 || row.lines_deleted > 0,
    input: row.input_tokens,
    output: row.output_tokens,
    cacheRead: row.cache_read_tokens,
    cacheCreation: row.cache_creation_tokens,
    totalContext: row.input_tokens + row.cache_read_tokens + row.cache_creation_tokens,
    cost: row.cost_usd,
  }));

  const empty = (
    <div className="h-48 flex items-center justify-center">
      <p className="text-muted-foreground text-sm">No data yet — run a task to see activity</p>
    </div>
  );

  const skeleton = <div className="h-48 animate-pulse bg-muted rounded" />;

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
      {/* Lines of Code */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Lines of Code</h3>
        <p className="text-xs text-muted-foreground">From OTel telemetry — only sessions with telemetry enabled</p>
        {isLoading ? skeleton : chartData.length === 0 ? empty : (
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={chartData} margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} label={{ value: 'Task', position: 'insideBottom', offset: -15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }} />
              <YAxis tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                labelFormatter={(_, payload) => {
                  const p = payload?.[0]?.payload;
                  if (!p) return '';
                  return `${p.fullTitle}${!p.hasLines ? ' (no OTel data)' : ''}`;
                }}
                formatter={(value, name) => [value, name === 'added' ? 'Lines added' : 'Lines deleted']}
                contentStyle={TOOLTIP_STYLE}
              />
              <Legend formatter={(v) => (v === 'added' ? 'Added' : 'Deleted')} iconType="circle" />
              <Bar dataKey="added" fill="#22c55e" name="added" radius={[3, 3, 0, 0]} />
              <Bar dataKey="deleted" fill="#ef4444" name="deleted" radius={[3, 3, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Token Usage */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Token Usage</h3>
        <p className="text-xs text-muted-foreground">Input + output tokens per task (from session logs)</p>
        {isLoading ? skeleton : chartData.length === 0 ? empty : (
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={chartData} margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} label={{ value: 'Task', position: 'insideBottom', offset: -15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }} />
              <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
                formatter={(value, name) => [formatTokens(Number(value)), name === 'input' ? 'Input tokens' : 'Output tokens']}
                contentStyle={TOOLTIP_STYLE}
              />
              <Legend formatter={(v) => (v === 'input' ? 'Input' : 'Output')} iconType="circle" />
              <Bar dataKey="input" fill="#6366f1" name="input" radius={[3, 3, 0, 0]} stackId="tokens" />
              <Bar dataKey="output" fill="#a855f7" name="output" radius={[3, 3, 0, 0]} stackId="tokens" />
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Context Size (Cache) */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Context Size (Cache)</h3>
        <p className="text-xs text-muted-foreground">Total context per task including prompt cache reads</p>
        {isLoading ? skeleton : chartData.length === 0 ? empty : (
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={chartData} margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} label={{ value: 'Task', position: 'insideBottom', offset: -15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }} />
              <YAxis tickFormatter={formatTokens} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
                formatter={(value, name) => [
                  formatTokens(Number(value)),
                  name === 'cacheRead' ? 'Cache reads' : 'Cache writes',
                ]}
                contentStyle={TOOLTIP_STYLE}
              />
              <Legend formatter={(v) => (v === 'cacheRead' ? 'Cache reads' : 'Cache writes')} iconType="circle" />
              <Bar dataKey="cacheRead" fill="#0ea5e9" name="cacheRead" radius={[3, 3, 0, 0]} stackId="ctx" />
              <Bar dataKey="cacheCreation" fill="#38bdf8" name="cacheCreation" radius={[3, 3, 0, 0]} stackId="ctx" />
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>
    </div>
  );
}
