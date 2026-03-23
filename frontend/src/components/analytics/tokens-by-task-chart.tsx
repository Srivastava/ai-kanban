'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend } from 'recharts';
import { useTokensByTask } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function truncate(s: string, n = 18): string {
  return s.length > n ? s.slice(0, n) + '\u2026' : s;
}

interface Props { taskId?: string | null }

export function TokensByTaskChart({ taskId }: Props) {
  const { data: allData = [], isLoading } = useTokensByTask();
  const data = taskId ? allData.filter((r) => r.task_id === taskId) : allData;

  const chartData = [...data]
    .sort((a, b) => {
      // Sort by input + output (the "work" tokens — cache overwhelms scale)
      const effectiveA = a.input_tokens + a.output_tokens;
      const effectiveB = b.input_tokens + b.output_tokens;
      return effectiveB - effectiveA;
    })
    .slice(0, 15)
    .map((d) => ({
      label: truncate(d.task_title),
      fullTitle: d.task_title,
      input: d.input_tokens,
      output: d.output_tokens,
      // cached kept for tooltip only — too large to show in bars
      cachedTooltip: (d.cache_creation_tokens ?? 0) + (d.cache_read_tokens ?? 0),
    }));

  const dynamicHeight = Math.max(240, chartData.length * 30);

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Tokens by Task</h3>
        {data.length > 15 && (
          <span className="text-xs text-muted-foreground">Top 15 shown</span>
        )}
      </div>
      {isLoading ? (
        <div className="h-64 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-64 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No task token data yet. Run a Claude session to see usage.</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={dynamicHeight}>
          <BarChart data={chartData} layout="vertical" margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" horizontal={false} />
            <XAxis
              type="number"
              tickFormatter={formatTokens}
              tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }}
            />
            <YAxis
              type="category"
              dataKey="label"
              width={130}
              tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }}
            />
            <Tooltip
              labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
              formatter={(value, name, props) => {
                const labels: Record<string, string> = { input: 'Input', output: 'Output' };
                const rows: [string, string][] = [[formatTokens(Number(value)), labels[String(name)] ?? String(name)]];
                // Append cached as extra info on the last series
                if (String(name) === 'output') {
                  const cached = props?.payload?.cachedTooltip ?? 0;
                  if (cached > 0) {
                    return [[formatTokens(Number(value)), 'Output'], [formatTokens(cached), 'Cached (ctx)']];
                  }
                }
                return rows;
              }}
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: '12px',
                color: 'hsl(var(--card-foreground))',
              }}
            />
            <Legend
              formatter={(v: string) => ({ input: 'Input', output: 'Output' }[v] ?? v)}
              iconType="circle"
              wrapperStyle={{ color: 'hsl(var(--muted-foreground))', fontSize: '11px' }}
            />
            <Bar dataKey="input" stackId="a" fill="#6366f1" name="input" />
            <Bar dataKey="output" stackId="a" fill="#a855f7" name="output" />
          </BarChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
