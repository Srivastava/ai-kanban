'use client';

import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid } from 'recharts';
import { useTokensByTool } from '@/hooks/use-analytics';

const COLORS = ['#6366f1', '#a855f7', '#ec4899', '#f97316', '#eab308', '#22c55e', '#06b6d4', '#f43f5e'];

function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

interface Props { taskId?: string | null }

export function ToolBreakdownChart({ taskId }: Props) {
  const { data = [], isLoading } = useTokensByTool(taskId);

  const top = data.slice(0, 8);
  const total = top.reduce((s, d) => s + d.input_tokens + d.output_tokens, 0);

  const pieData = top.map((d) => ({
    name: d.tool_name,
    value: d.input_tokens + d.output_tokens,
    calls: d.call_count,
  }));

  const barData = [...top]
    .sort((a, b) => b.call_count - a.call_count)
    .map((d, i) => ({ name: d.tool_name, calls: d.call_count, colorIdx: top.indexOf(d) }));

  const TOOLTIP_STYLE = {
    background: 'hsl(var(--card))',
    border: '1px solid hsl(var(--border))',
    borderRadius: '8px',
    fontSize: '12px',
    color: 'hsl(var(--card-foreground))',
  };

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-5">
      <h3 className="font-semibold">Tool Usage</h3>

      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : pieData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No tool data yet</p>
        </div>
      ) : (
        <>
          {/* Token share donut + legend */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 items-center">
            <div>
              <p className="text-xs text-muted-foreground uppercase tracking-wider mb-2">Token share</p>
              <ResponsiveContainer width="100%" height={200}>
                <PieChart>
                  <Pie
                    data={pieData}
                    cx="50%"
                    cy="50%"
                    innerRadius={52}
                    outerRadius={80}
                    paddingAngle={2}
                    dataKey="value"
                    nameKey="name"
                    stroke="hsl(var(--card))"
                    strokeWidth={2}
                  >
                    {pieData.map((_, i) => (
                      <Cell key={i} fill={COLORS[i % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value, name) => [
                      `${fmtTokens(Number(value))} (${total > 0 ? ((Number(value) / total) * 100).toFixed(1) : 0}%)`,
                      String(name),
                    ]}
                    contentStyle={TOOLTIP_STYLE}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>

            {/* Legend with both tokens and calls */}
            <div className="space-y-1.5">
              {pieData.map((entry, i) => (
                <div key={entry.name} className="flex items-center gap-2 text-xs">
                  <span className="w-2.5 h-2.5 rounded-sm flex-shrink-0" style={{ background: COLORS[i % COLORS.length] }} />
                  <span className="flex-1 text-foreground truncate">{entry.name}</span>
                  <span className="tabular-nums text-muted-foreground">{fmtTokens(entry.value)}</span>
                  <span className="tabular-nums text-muted-foreground w-14 text-right">{entry.calls.toLocaleString()} calls</span>
                </div>
              ))}
            </div>
          </div>

          {/* Call frequency bar chart */}
          <div>
            <p className="text-xs text-muted-foreground uppercase tracking-wider mb-2">Call frequency</p>
            <ResponsiveContainer width="100%" height={Math.max(120, barData.length * 28)}>
              <BarChart data={barData} layout="vertical" margin={{ top: 0, right: 40, left: 8, bottom: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" horizontal={false} />
                <XAxis type="number" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
                <YAxis
                  type="category"
                  dataKey="name"
                  width={80}
                  tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                />
                <Tooltip
                  formatter={(v) => [Number(v).toLocaleString(), 'Calls']}
                  contentStyle={TOOLTIP_STYLE}
                />
                <Bar dataKey="calls" radius={[0, 3, 3, 0]}>
                  {barData.map((entry, i) => (
                    <Cell key={i} fill={COLORS[entry.colorIdx % COLORS.length]} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        </>
      )}
    </div>
  );
}
