'use client';

import { useRoiMetrics } from '@/hooks/use-analytics';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { TokenEfficiencyChart } from './token-efficiency-chart';

interface Props { taskId?: string | null }

function formatSecs(s: number) {
  if (s < 3600) return `${Math.round(s / 60)}m`;
  return `${(s / 3600).toFixed(1)}h`;
}

export function ProductivitySection({ taskId }: Props) {
  const { data: roi } = useRoiMetrics(taskId);

  const hasOtelData = (roi?.total_commits ?? 0) > 0 || (roi?.total_prs ?? 0) > 0;

  const activityData = roi ? [
    { name: 'Commits', value: roi.total_commits, color: '#6366f1' },
    { name: 'PRs', value: roi.total_prs, color: '#8b5cf6' },
    { name: 'LOC (÷100)', value: Math.round(roi.total_loc / 100), color: '#a78bfa' },
  ] : [];

  return (
    <div className="space-y-6">
      {!hasOtelData && (
        <div className="rounded-xl border border-border bg-card/50 p-4 text-sm text-muted-foreground">
          Commit, PR, and active-time data appears once Claude Code reports OTel metrics.
          Ensure <code className="text-xs bg-muted px-1 py-0.5 rounded">OTEL_EXPORTER_OTLP_ENDPOINT</code> points
          to <code className="text-xs bg-muted px-1 py-0.5 rounded">http://localhost:4318</code>.
        </div>
      )}

      {hasOtelData && (
        <div className="rounded-xl border border-border bg-card p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">Output Activity</h3>
            {roi && (
              <span className="text-xs text-muted-foreground">
                {formatSecs(roi.total_active_time_secs)} active time
              </span>
            )}
          </div>
          <ResponsiveContainer width="100%" height={160}>
            <BarChart data={activityData} barSize={40}>
              <XAxis dataKey="name" tick={{ fontSize: 12, fill: 'hsl(var(--muted-foreground))' }} />
              <YAxis tick={{ fontSize: 12, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                formatter={(v: number | undefined) => [v ?? 0, '']}
                contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: 6, fontSize: 12, color: 'hsl(var(--card-foreground))' }}
              />
              <Bar dataKey="value" radius={[4, 4, 0, 0]}>
                {activityData.map((entry) => (
                  <Cell key={entry.name} fill={entry.color} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Token efficiency — reuse existing component */}
      <TokenEfficiencyChart taskId={taskId} />
    </div>
  );
}
