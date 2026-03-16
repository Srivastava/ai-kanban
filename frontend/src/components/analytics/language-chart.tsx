'use client';

import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';
import { useTokensByLanguage } from '@/hooks/use-analytics';

const COLORS = ['#6366f1', '#a855f7', '#ec4899', '#f97316', '#eab308', '#22c55e', '#06b6d4', '#f43f5e'];

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

interface Props { taskId?: string | null }

export function LanguageChart({ taskId }: Props) {
  const { data = [], isLoading } = useTokensByLanguage(taskId);

  const top = data.slice(0, 7);
  const otherTokens = data.slice(7).reduce((s, d) => s + d.input_tokens + d.output_tokens, 0);
  const chartData = [
    ...top.map((d) => ({ ext: d.file_ext, value: d.input_tokens + d.output_tokens, calls: d.call_count })),
    ...(otherTokens > 0 ? [{ ext: 'other', value: otherTokens, calls: 0 }] : []),
  ];
  const total = chartData.reduce((s, d) => s + d.value, 0);

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Tokens by Language</h3>
      {isLoading ? (
        <div className="h-56 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-56 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No language data yet</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 items-center">
          {/* Donut */}
          <ResponsiveContainer width="100%" height={220}>
            <PieChart>
              <Pie
                data={chartData}
                cx="50%"
                cy="50%"
                innerRadius={60}
                outerRadius={90}
                paddingAngle={2}
                dataKey="value"
                nameKey="ext"
                stroke="hsl(var(--card))"
                strokeWidth={2}
              >
                {chartData.map((_, i) => (
                  <Cell key={i} fill={COLORS[i % COLORS.length]} />
                ))}
              </Pie>
              <Tooltip
                formatter={(value, name) => [
                  `${formatTokens(Number(value))} (${total > 0 ? ((Number(value) / total) * 100).toFixed(1) : 0}%)`,
                  String(name),
                ]}
                contentStyle={{
                  background: 'hsl(var(--card))',
                  border: '1px solid hsl(var(--border))',
                  borderRadius: '8px',
                  fontSize: '12px',
                }}
              />
            </PieChart>
          </ResponsiveContainer>

          {/* Legend table */}
          <div className="space-y-2">
            {chartData.map((entry, i) => (
              <div key={entry.ext} className="flex items-center gap-2 text-sm">
                <span
                  className="w-3 h-3 rounded-sm flex-shrink-0"
                  style={{ background: COLORS[i % COLORS.length] }}
                />
                <span className="flex-1 font-mono text-foreground">{entry.ext}</span>
                <span className="tabular-nums text-muted-foreground">{formatTokens(entry.value)}</span>
                <span className="tabular-nums text-xs text-muted-foreground w-12 text-right">
                  {total > 0 ? `${((entry.value / total) * 100).toFixed(0)}%` : '—'}
                </span>
              </div>
            ))}
            <div className="pt-1 border-t border-border flex justify-between text-xs text-muted-foreground">
              <span>Total</span>
              <span className="tabular-nums font-medium">{formatTokens(total)}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
