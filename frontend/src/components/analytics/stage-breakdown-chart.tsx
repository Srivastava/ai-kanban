'use client';

import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { useTokensByStage } from '@/hooks/use-analytics';

const STAGE_ORDER = ['backlog', 'planning', 'in_progress', 'review', 'done'];

const STAGE_COLORS: Record<string, string> = {
  backlog: '#94a3b8',
  planning: '#6366f1',
  in_progress: '#22c55e',
  review: '#f97316',
  done: '#a855f7',
};

function stageColor(stage: string): string {
  return STAGE_COLORS[stage] ?? '#64748b';
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function stageLabel(s: string): string {
  return s === 'in_progress' ? 'In Progress' : s.charAt(0).toUpperCase() + s.slice(1);
}

interface Props { taskId?: string | null }

export function StageBreakdownChart({ taskId }: Props) {
  const { data = [], isLoading } = useTokensByStage(taskId);

  const chartData = [...data]
    .sort((a, b) =>
      (STAGE_ORDER.indexOf(a.stage) + 1 || 99) - (STAGE_ORDER.indexOf(b.stage) + 1 || 99)
    )
    .map((d) => ({
      stage: d.stage,
      label: stageLabel(d.stage),
      value: d.input_tokens + d.output_tokens,
      input: d.input_tokens,
      output: d.output_tokens,
    }))
    .filter((d) => d.value > 0);

  const total = chartData.reduce((s, d) => s + d.value, 0);

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div>
        <h3 className="font-semibold">Tokens by Stage</h3>
        <p className="text-xs text-muted-foreground mt-0.5">Stage task was in when AI worked on it</p>
      </div>
      {isLoading ? (
        <div className="h-56 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-56 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No stage data yet</p>
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
                nameKey="label"
                stroke="hsl(var(--card))"
                strokeWidth={2}
              >
                {chartData.map((entry) => (
                  <Cell key={entry.stage} fill={stageColor(entry.stage)} />
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
            {chartData.map((entry) => (
              <div key={entry.stage} className="flex items-center gap-2 text-sm">
                <span
                  className="w-3 h-3 rounded-sm flex-shrink-0"
                  style={{ background: stageColor(entry.stage) }}
                />
                <span className="flex-1 text-foreground">{entry.label}</span>
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
