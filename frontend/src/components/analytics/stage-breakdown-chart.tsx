'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend } from 'recharts';
import { useTokensByStage } from '@/hooks/use-analytics';

const STAGE_ORDER = ['backlog', 'planning', 'in_progress', 'review', 'done'];

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function StageBreakdownChart() {
  const { data = [], isLoading } = useTokensByStage();

  const chartData = [...data]
    .sort((a, b) =>
      (STAGE_ORDER.indexOf(a.stage) + 1 || 99) - (STAGE_ORDER.indexOf(b.stage) + 1 || 99)
    )
    .map((d) => ({ stage: d.stage, input: d.input_tokens, output: d.output_tokens }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Tokens by Stage</h3>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No stage data yet</p>
        </div>
      ) : (
        <>
          <ResponsiveContainer width="100%" height={224}>
            <BarChart data={chartData} layout="vertical" margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" horizontal={false} />
              <XAxis type="number" tickFormatter={formatTokens} tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} label={{ value: 'Tokens', position: 'insideBottom', offset: -15, style: { fontSize: 11, fill: 'hsl(var(--muted-foreground))' } }} />
              <YAxis type="category" dataKey="stage" width={80} tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} label={{ value: 'Stage', angle: -90, position: 'insideLeft', offset: 15, style: { fontSize: 11, fill: 'hsl(var(--muted-foreground))' } }} />
              <Tooltip
                formatter={(value, name) => [
                  formatTokens(Number(value)),
                  name === 'input' ? 'Input tokens' : 'Output tokens',
                ]}
                contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '8px', fontSize: '12px' }}
              />
              <Legend formatter={(v) => (v === 'input' ? 'Input' : 'Output')} iconType="circle" />
              <Bar dataKey="input" stackId="a" fill="#6366f1" name="input" />
              <Bar dataKey="output" stackId="a" fill="#a855f7" name="output" />
            </BarChart>
          </ResponsiveContainer>
          <ul className="flex flex-wrap gap-x-4 gap-y-1">
            {chartData.map((entry) => (
              <li key={entry.stage} className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <span className="inline-block w-2 h-2 rounded-full bg-indigo-500 flex-shrink-0" />
                {entry.stage}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
