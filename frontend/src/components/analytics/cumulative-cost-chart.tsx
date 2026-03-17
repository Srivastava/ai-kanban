'use client';

import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useDailyTokens } from '@/hooks/use-analytics';

// Pricing (must match backend token_prices defaults)
const PRICE = {
  input: 3.0 / 1_000_000,
  output: 15.0 / 1_000_000,
  cacheWrite: 3.75 / 1_000_000,
  cacheRead: 0.30 / 1_000_000,
};

function dayCost(d: { input_tokens: number; output_tokens: number; cache_creation_tokens: number; cache_read_tokens: number }): number {
  return d.input_tokens * PRICE.input
    + d.output_tokens * PRICE.output
    + d.cache_creation_tokens * PRICE.cacheWrite
    + d.cache_read_tokens * PRICE.cacheRead;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

interface Props { taskId?: string | null }

export function CumulativeCostChart({ taskId }: Props) {
  const { data: daily = [], isLoading } = useDailyTokens(30, taskId);

  let running = 0;
  const chartData = daily.map((d) => {
    const cost = dayCost(d);
    running += cost;
    return {
      date: d.date.slice(5), // strip year: "03-17"
      cumulative: parseFloat(running.toFixed(4)),
    };
  });

  const totalCost = running;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="font-semibold">Cumulative Cost (30 days)</h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {taskId ? 'Running spend for selected task' : 'Running spend across all tasks'}
          </p>
        </div>
        {!isLoading && totalCost > 0 && (
          <span className="text-lg font-bold tabular-nums text-emerald-500">
            ${totalCost.toFixed(4)}
          </span>
        )}
      </div>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No cost data in the last 30 days</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={200}>
          <AreaChart data={chartData} margin={{ top: 5, right: 10, left: 10, bottom: 5 }}>
            <defs>
              <linearGradient id="costGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#10b981" stopOpacity={0.02} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
            <XAxis dataKey="date" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
            <YAxis
              tickFormatter={(v) => `$${v.toFixed(2)}`}
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
            />
            <Tooltip
              formatter={(value) => [`$${Number(value).toFixed(4)}`, 'Cumulative']}
              contentStyle={TOOLTIP_STYLE}
            />
            <Area
              type="monotone"
              dataKey="cumulative"
              stroke="#10b981"
              strokeWidth={2}
              fill="url(#costGradient)"
            />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
