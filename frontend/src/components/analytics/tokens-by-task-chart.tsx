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

export function TokensByTaskChart() {
  const { data = [], isLoading } = useTokensByTask();

  const chartData = [...data]
    .sort((a, b) => {
      // Sort by effective total (what the bar width represents)
      const effectiveA = a.input_tokens + (a.cache_creation_tokens ?? 0) + (a.cache_read_tokens ?? 0) + a.output_tokens;
      const effectiveB = b.input_tokens + (b.cache_creation_tokens ?? 0) + (b.cache_read_tokens ?? 0) + b.output_tokens;
      return effectiveB - effectiveA;
    })
    .slice(0, 15)
    .map((d) => ({
      label: truncate(d.task_title),
      fullTitle: d.task_title,
      input: d.input_tokens,
      cached: (d.cache_creation_tokens ?? 0) + (d.cache_read_tokens ?? 0),
      output: d.output_tokens,
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
              formatter={(value, name) => {
                const labels: Record<string, string> = { input: 'Input', cached: 'Cached', output: 'Output' };
                return [formatTokens(Number(value)), labels[String(name)] ?? String(name)];
              }}
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: '12px',
              }}
            />
            <Legend formatter={(v: string) => ({ input: 'Input', cached: 'Cached', output: 'Output' }[v] ?? v)} iconType="circle" />
            <Bar dataKey="input" stackId="a" fill="#6366f1" name="input" />
            <Bar dataKey="cached" stackId="a" fill="#f59e0b" name="cached" />
            <Bar dataKey="output" stackId="a" fill="#a855f7" name="output" />
          </BarChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
