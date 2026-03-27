'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend } from 'recharts';
import { useTokensByTask } from '@/hooks/use-analytics';
import { formatTokens } from '@/lib/format';
import { TOKEN_COLORS } from '@/lib/chart-colors';

function truncate(s: string, n = 18): string {
  return s.length > n ? s.slice(0, n) + '\u2026' : s;
}

interface Props { taskId?: string | null }

export function TokensByTaskChart({ taskId }: Props) {
  const { data: allData = [], isLoading } = useTokensByTask();
  const data = taskId ? allData.filter((r) => r.task_id === taskId) : allData;

  const chartData = [...data]
    .sort((a, b) => {
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
      cachedTooltip: (d.cache_creation_tokens ?? 0) + (d.cache_read_tokens ?? 0),
    }));

  const dynamicHeight = Math.max(240, chartData.length * 30);

  const TOOLTIP_STYLE = {
    background: 'var(--card)',
    border: '1px solid var(--border)',
    borderRadius: '8px',
    fontSize: '12px',
    color: 'var(--card-foreground)',
  };

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Tokens by Task</h3>
        {data.length > 15 && (
          <span className="text-xs text-muted-foreground">Top 15 shown</span>
        )}
      </div>
      {isLoading ? (
        <div className="h-64 bg-muted rounded animate-shimmer" />
      ) : chartData.length === 0 ? (
        <div className="h-64 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No task token data yet. Run a Claude session to see usage.</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={dynamicHeight}>
          <BarChart data={chartData} layout="vertical" margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" horizontal={false} />
            <XAxis
              type="number"
              tickFormatter={formatTokens}
              tick={{ fontSize: 11, fill: 'var(--muted-foreground)' }}
            />
            <YAxis
              type="category"
              dataKey="label"
              width={130}
              tick={{ fontSize: 11, fill: 'var(--muted-foreground)' }}
            />
            <Tooltip
              labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
              formatter={(value, name, props) => {
                const labels: Record<string, string> = { input: 'Input', output: 'Output' };
                const rows: [string, string][] = [[formatTokens(Number(value)), labels[String(name)] ?? String(name)]];
                if (String(name) === 'output') {
                  const cached = props?.payload?.cachedTooltip ?? 0;
                  if (cached > 0) {
                    return [[formatTokens(Number(value)), 'Output'], [formatTokens(cached), 'Cached (ctx)']];
                  }
                }
                return rows;
              }}
              contentStyle={TOOLTIP_STYLE}
            />
            <Legend
              formatter={(v: string) => ({ input: 'Input', output: 'Output' }[v] ?? v)}
              iconType="circle"
              wrapperStyle={{ color: 'var(--muted-foreground)', fontSize: '11px' }}
            />
            <Bar dataKey="input"  stackId="a" fill={TOKEN_COLORS.input}  name="input" />
            <Bar dataKey="output" stackId="a" fill={TOKEN_COLORS.output} name="output" />
          </BarChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
