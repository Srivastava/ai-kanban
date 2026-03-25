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

  const stats = [
    {
      label: 'Cost / Commit',
      value: isLoading ? null : formatCost(roi?.cost_per_commit ?? null),
      sub: roi ? `${roi.total_commits} commits` : null,
    },
    {
      label: 'Cost / PR',
      value: isLoading ? null : formatCost(roi?.cost_per_pr ?? null),
      sub: roi ? `${roi.total_prs} PRs` : null,
    },
    {
      label: 'Cost / 100 Lines',
      value: isLoading ? null : (
        roi?.cost_per_loc != null ? `$${(roi.cost_per_loc * 100).toFixed(3)}` : '—'
      ),
      sub: roi ? `${roi.total_loc.toLocaleString()} lines` : null,
    },
    {
      label: 'Avg Session',
      value: isLoading ? null : (roi ? formatSecs(roi.avg_session_duration_secs) : '—'),
      sub: roi ? `${formatSecs(roi.total_active_time_secs)} total` : null,
    },
  ];

  return (
    <div className="rounded-xl border border-border bg-card divide-y sm:divide-y-0 sm:divide-x divide-border sm:flex">
      {stats.map((s) => (
        <div key={s.label} className="flex-1 px-4 py-3 sm:py-4 flex sm:flex-col items-center sm:items-start justify-between gap-1">
          <p className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground whitespace-nowrap">
            {s.label}
          </p>
          <div className="text-right sm:text-left">
            <p className="text-lg font-bold tabular-nums leading-none">
              {s.value === null
                ? <span className="inline-block w-12 h-5 rounded motion-safe:animate-pulse bg-muted" />
                : s.value}
            </p>
            {s.sub && (
              <p className="text-[11px] text-muted-foreground mt-0.5">{s.sub}</p>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
