'use client';

import { useState } from 'react';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { useDailyTokens, useWeeklyTokens, useMonthlyTokens } from '@/hooks/use-analytics';

type Period = 'daily' | 'weekly' | 'monthly';

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

export function TokenTimeChart() {
  const [period, setPeriod] = useState<Period>('daily');

  const daily = useDailyTokens(30);
  const weekly = useWeeklyTokens(12);
  const monthly = useMonthlyTokens(6);

  const dataMap = {
    daily: (daily.data ?? []).map((d) => ({ label: d.date.slice(5), input: d.input_tokens, output: d.output_tokens })),
    weekly: (weekly.data ?? []).map((d) => ({ label: d.week_start.slice(5), input: d.input_tokens, output: d.output_tokens })),
    monthly: (monthly.data ?? []).map((d) => ({ label: d.month, input: d.input_tokens, output: d.output_tokens })),
  };

  const data = dataMap[period];
  const isLoading = { daily, weekly, monthly }[period].isLoading;
  const xLabel = { daily: 'Date', weekly: 'Week', monthly: 'Month' }[period];

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Token Usage Over Time</h3>
        <div className="flex gap-1">
          {(['daily', 'weekly', 'monthly'] as Period[]).map((p) => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                period === p ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:bg-muted'
              }`}
            >
              {p.charAt(0).toUpperCase() + p.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="h-64 flex items-center justify-center"><div className="animate-pulse text-muted-foreground text-sm">Loading...</div></div>
      ) : data.length === 0 ? (
        <div className="h-64 flex items-center justify-center"><p className="text-muted-foreground text-sm">No token data yet. Run a Claude session to see usage.</p></div>
      ) : (
        <ResponsiveContainer width="100%" height={256}>
          <AreaChart data={data} margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
            <defs>
              <linearGradient id="inputGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="outputGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#a855f7" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#a855f7" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis
              dataKey="label"
              tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }}
              label={{ value: xLabel, position: 'insideBottom', offset: -15, style: { fontSize: 11, fill: 'hsl(var(--muted-foreground))' } }}
            />
            <YAxis
              tickFormatter={formatTokens}
              tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }}
              label={{ value: 'Tokens', angle: -90, position: 'insideLeft', offset: 15, style: { fontSize: 11, fill: 'hsl(var(--muted-foreground))' } }}
            />
            <Tooltip
              formatter={(value, name) => [formatTokens(Number(value)), name === 'input' ? 'Input tokens' : 'Output tokens']}
              contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '8px', fontSize: '12px' }}
            />
            <Legend formatter={(value) => (value === 'input' ? 'Input' : 'Output')} iconType="circle" />
            <Area type="monotone" dataKey="input" stroke="#6366f1" fill="url(#inputGrad)" strokeWidth={2} dot={false} />
            <Area type="monotone" dataKey="output" stroke="#a855f7" fill="url(#outputGrad)" strokeWidth={2} dot={false} />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
