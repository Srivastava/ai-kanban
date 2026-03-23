'use client';

import { useState } from 'react';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { useDailyTokens, useWeeklyTokens, useMonthlyTokens } from '@/hooks/use-analytics';
import { TokenTrendBadges } from '@/components/analytics/period-comparison';

type Period = 'daily' | 'weekly' | 'monthly';

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

const LABEL_MAP: Record<string, string> = {
  input: 'Input',
  cached: 'Cached (→)',
  output: 'Output',
};

interface Props { taskId?: string | null }

export function TokenTimeChart({ taskId }: Props) {
  const [period, setPeriod] = useState<Period>('daily');

  const daily = useDailyTokens(30, taskId);
  const weekly = useWeeklyTokens(12, taskId);
  const monthly = useMonthlyTokens(6, taskId);

  const dataMap = {
    daily: (daily.data ?? []).map((d) => ({
      label: d.date.slice(5),
      input: d.input_tokens,
      cached: (d.cache_creation_tokens ?? 0) + (d.cache_read_tokens ?? 0),
      output: d.output_tokens,
    })),
    weekly: (weekly.data ?? []).map((d) => ({
      label: d.week_start.slice(5),
      input: d.input_tokens,
      cached: (d.cache_creation_tokens ?? 0) + (d.cache_read_tokens ?? 0),
      output: d.output_tokens,
    })),
    monthly: (monthly.data ?? []).map((d) => ({
      label: d.month,
      input: d.input_tokens,
      cached: (d.cache_creation_tokens ?? 0) + (d.cache_read_tokens ?? 0),
      output: d.output_tokens,
    })),
  };

  const data = dataMap[period];
  const isLoading = { daily, weekly, monthly }[period].isLoading;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between gap-2 flex-wrap">
        <div className="flex items-center gap-2 flex-wrap">
          <h3 className="font-semibold">Token Usage Over Time</h3>
          <TokenTrendBadges />
        </div>
        <div className="flex gap-1">
          {(['daily', 'weekly', 'monthly'] as const).map((p) => (
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
      </div>

      {isLoading ? (
        <div className="h-64 flex items-center justify-center"><div className="animate-pulse text-muted-foreground text-sm">Loading...</div></div>
      ) : data.length === 0 ? (
        <div className="h-64 flex items-center justify-center"><p className="text-muted-foreground text-sm">No token data yet. Run a Claude session to see usage.</p></div>
      ) : (
        <ResponsiveContainer width="100%" height={256}>
          <AreaChart data={data} margin={{ top: 5, right: 55, left: 10, bottom: 5 }}>
            <defs>
              <linearGradient id="inputGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="cachedGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#f59e0b" stopOpacity={0.2} />
                <stop offset="95%" stopColor="#f59e0b" stopOpacity={0} />
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
            />
            {/* Left axis: input + output */}
            <YAxis
              yAxisId="left"
              tickFormatter={formatTokens}
              tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }}
            />
            {/* Right axis: cached (much larger scale) */}
            <YAxis
              yAxisId="right"
              orientation="right"
              tickFormatter={formatTokens}
              tick={{ fontSize: 11, fill: '#f59e0b99' }}
            />
            <Tooltip
              formatter={(value, name) => [formatTokens(Number(value)), LABEL_MAP[String(name)] ?? String(name)]}
              contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '8px', fontSize: '12px', color: 'hsl(var(--card-foreground))' }}
            />
            <Legend formatter={(value) => LABEL_MAP[value] ?? value} iconType="circle" wrapperStyle={{ color: 'hsl(var(--muted-foreground))', fontSize: '11px' }} />
            <Area yAxisId="left" type="monotone" dataKey="input" stroke="#6366f1" fill="url(#inputGrad)" strokeWidth={2} dot={false} />
            <Area yAxisId="left" type="monotone" dataKey="output" stroke="#a855f7" fill="url(#outputGrad)" strokeWidth={2} dot={false} />
            <Area yAxisId="right" type="monotone" dataKey="cached" stroke="#f59e0b" fill="url(#cachedGrad)" strokeWidth={1.5} dot={false} strokeDasharray="4 2" />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
