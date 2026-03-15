'use client';

import { useRoiMetrics } from '@/hooks/use-analytics';

function formatCost(v: number | null) {
  if (v === null) return '—';
  if (v < 0.01) return `$${v.toFixed(4)}`;
  return `$${v.toFixed(2)}`;
}

function formatSecs(s: number) {
  if (s < 60) return `${Math.round(s)}s`;
  if (s < 3600) return `${Math.round(s / 60)}m`;
  return `${(s / 3600).toFixed(1)}h`;
}

interface Props { taskId?: string | null }

export function RoiCards({ taskId }: Props) {
  const { data: roi, isLoading } = useRoiMetrics(taskId);

  const cards = [
    {
      label: 'Cost / Commit',
      value: isLoading ? null : formatCost(roi?.cost_per_commit ?? null),
      sub: roi ? `${roi.total_commits} commits` : '',
    },
    {
      label: 'Cost / PR',
      value: isLoading ? null : formatCost(roi?.cost_per_pr ?? null),
      sub: roi ? `${roi.total_prs} PRs` : '',
    },
    {
      label: 'Cost / 100 Lines',
      value: isLoading ? null : (
        roi?.cost_per_loc != null ? `$${(roi.cost_per_loc * 100).toFixed(3)}` : '—'
      ),
      sub: roi ? `${roi.total_loc.toLocaleString()} net lines` : '',
    },
    {
      label: 'Avg Session',
      value: isLoading ? null : (roi ? formatSecs(roi.avg_session_duration_secs) : '—'),
      sub: roi ? `${formatSecs(roi.total_active_time_secs)} active` : '',
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {cards.map((c) => (
        <div key={c.label} className="rounded-xl border border-border bg-card p-4 space-y-1">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">{c.label}</p>
          <p className="text-2xl font-bold">
            {c.value === null
              ? <span className="inline-block w-16 h-7 rounded animate-pulse bg-muted" />
              : c.value}
          </p>
          <p className="text-xs text-muted-foreground">{c.sub}</p>
        </div>
      ))}
    </div>
  );
}
