'use client';

import { useMemo, useId } from 'react';
import { AreaChart, Area, XAxis, Tooltip, ResponsiveContainer } from 'recharts';
import type { LogEntry } from '@/types/log';

const TZ = 'America/Los_Angeles';

function bucketByMinute(logs: LogEntry[]) {
  const buckets = new Map<string, { total: number; errors: number }>();

  for (const log of logs) {
    const d = new Date(log.timestamp);
    const label = new Intl.DateTimeFormat('en-US', {
      timeZone: TZ,
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    }).format(d);

    const existing = buckets.get(label) ?? { total: 0, errors: 0 };
    existing.total += 1;
    if (log.level === 'ERROR') existing.errors += 1;
    buckets.set(label, existing);
  }

  return Array.from(buckets.entries())
    .map(([label, v]) => ({ label, ...v }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

interface Props {
  logs: LogEntry[];
}

export function LogRateChart({ logs }: Props) {
  const gradId = useId();
  const data = useMemo(() => bucketByMinute(logs), [logs]);

  if (data.length < 2) return null;

  const hasErrors = data.some((d) => d.errors > 0);

  return (
    <div className="rounded-xl border border-border bg-card/50 px-4 pt-3 pb-2">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-muted-foreground">Log rate (per minute)</span>
        <span className="text-xs text-muted-foreground">{data.length} min window</span>
      </div>
      <ResponsiveContainer width="100%" height={64}>
        <AreaChart data={data} margin={{ top: 2, right: 2, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={hasErrors ? '#ef4444' : '#6366f1'} stopOpacity={0.3} />
              <stop offset="95%" stopColor={hasErrors ? '#ef4444' : '#6366f1'} stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis dataKey="label" hide />
          <Tooltip
            contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: '6px', fontSize: '11px' }}
            formatter={(value, name) => [value, name === 'total' ? 'logs' : 'errors']}
          />
          <Area type="monotone" dataKey="total" stroke={hasErrors ? '#ef4444' : '#6366f1'}
            fill={`url(#${gradId})`} strokeWidth={1.5} dot={false} />
          {hasErrors && (
            <Area type="monotone" dataKey="errors" stroke="#ef4444"
              fill="none" strokeWidth={1} strokeDasharray="3 3" dot={false} />
          )}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
