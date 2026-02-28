'use client';

import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';
import { useTokensByTool } from '@/hooks/use-analytics';

const COLORS = ['#6366f1', '#a855f7', '#ec4899', '#f97316', '#eab308', '#22c55e', '#06b6d4'];

export function ToolBreakdownChart() {
  const { data = [], isLoading } = useTokensByTool();

  const chartData = data.map((d) => ({ name: d.tool_name, value: d.input_tokens + d.output_tokens, calls: d.call_count }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Tokens per Tool Call</h3>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center"><p className="text-muted-foreground text-sm">No tool data yet</p></div>
      ) : (
        <ResponsiveContainer width="100%" height={192}>
          <PieChart>
            <Pie data={chartData} cx="50%" cy="50%" innerRadius={50} outerRadius={80} paddingAngle={3} dataKey="value">
              {chartData.map((_, i) => (<Cell key={i} fill={COLORS[i % COLORS.length]} />))}
            </Pie>
            <Tooltip formatter={(value) => [Number(value).toLocaleString() + ' tokens']} contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '8px', fontSize: '12px' }} />
          </PieChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
