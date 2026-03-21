'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { useHourlyBreakdown } from '@/hooks/use-analytics';

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

function hourLabel(h: number) {
  if (h === 0) return '12a';
  if (h < 12) return `${h}a`;
  if (h === 12) return '12p';
  return `${h - 12}p`;
}

interface Props { taskId?: string | null }

export function HourlyBreakdown({ taskId }: Props) {
  const { data = [], isLoading } = useHourlyBreakdown(taskId);

  // Fill all 24 hours, default 0
  const filled = Array.from({ length: 24 }, (_, h) => ({
    hour: h,
    label: hourLabel(h),
    tokens: data.find(d => d.hour === h)?.tokens ?? 0,
  }));

  const max = Math.max(...filled.map(d => d.tokens), 1);

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <h3 className="font-semibold text-sm">Activity by Hour</h3>
      {isLoading ? (
        <div className="h-32 animate-pulse bg-muted rounded" />
      ) : (
        <ResponsiveContainer width="100%" height={120}>
          <BarChart data={filled} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
            <XAxis dataKey="label" tick={{ fontSize: 8, fill: 'hsl(var(--muted-foreground))' }}
              interval={2} />
            <YAxis tickFormatter={fmt} tick={{ fontSize: 8, fill: 'hsl(var(--muted-foreground))' }} />
            <Tooltip
              formatter={(v) => [fmt(Number(v)), 'Tokens']}
              labelFormatter={(l) => `Hour: ${l} UTC`}
              contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: 6, fontSize: 11 }}
            />
            <Bar dataKey="tokens" radius={[2, 2, 0, 0]}>
              {filled.map((entry) => (
                <Cell key={entry.hour}
                  fill={entry.tokens === 0
                    ? 'hsl(var(--muted))'
                    : `hsl(239 84% ${30 + Math.round((entry.tokens / max) * 35)}%)`} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      )}
      <p className="text-[10px] text-muted-foreground">UTC hours · all time</p>
    </div>
  );
}
