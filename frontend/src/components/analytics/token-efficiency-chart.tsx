'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Cell } from 'recharts';
import { useTokenEfficiency } from '@/hooks/use-analytics';

function truncate(s: string, n = 14): string {
  return s.length > n ? s.slice(0, n) + '\u2026' : s;
}

// Gradient from efficient (indigo) to costly (amber) — sorted low-to-high so left = best
const COLORS = ['#6366f1', '#818cf8', '#a5b4fc', '#c084fc', '#e879f9', '#f59e0b'];

function colorFor(idx: number, total: number): string {
  const pos = Math.floor((idx / Math.max(total - 1, 1)) * (COLORS.length - 1));
  return COLORS[pos];
}

export function TokenEfficiencyChart() {
  const { data = [], isLoading } = useTokenEfficiency();

  const chartData = data
    .filter((d) => d.tokens_per_line != null && d.tokens_per_line > 0 && d.lines_written > 0)
    .sort((a, b) => (a.tokens_per_line ?? 0) - (b.tokens_per_line ?? 0))
    .map((d) => ({
      label: truncate(d.task_title),
      fullTitle: d.task_title,
      tpl: Math.round(d.tokens_per_line ?? 0),
      lines: d.lines_written,
      total: d.total_tokens,
    }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div>
        <h3 className="font-semibold">Token Efficiency</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Tokens per line of code written — lower is more efficient
        </p>
      </div>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No efficiency data yet</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={220}>
          <BarChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
            <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
            <YAxis
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
              label={{
                value: 'tokens/line',
                angle: -90,
                position: 'insideLeft',
                style: { fontSize: 9, fill: 'hsl(var(--muted-foreground))' },
                offset: 10,
              }}
            />
            <Tooltip
              labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
              formatter={(value, _name, props) => [
                [`${value} tokens/line`, `${props.payload.lines} lines written`, `${props.payload.total.toLocaleString()} total tokens`],
                '',
              ]}
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: '12px',
              }}
            />
            <Bar dataKey="tpl" name="tokens/line" radius={[3, 3, 0, 0]}>
              {chartData.map((_, idx) => (
                <Cell key={idx} fill={colorFor(idx, chartData.length)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
