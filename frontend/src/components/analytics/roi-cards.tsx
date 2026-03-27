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

  // Three cost metrics together, then the time metric visually separated
  const costStats = [
    {
      label: 'Cost / Commit',
      value: isLoading ? null : formatCost(roi?.cost_per_commit ?? null),
      sub: roi ? `${roi.total_commits} commits` : null,
      featured: true,
    },
    {
      label: 'Cost / PR',
      value: isLoading ? null : formatCost(roi?.cost_per_pr ?? null),
      sub: roi ? `${roi.total_prs} PRs` : null,
      featured: false,
    },
    {
      label: 'Cost / 100 Lines',
      value: isLoading ? null : (
        roi?.cost_per_loc != null ? `$${(roi.cost_per_loc * 100).toFixed(3)}` : '—'
      ),
      sub: roi ? `${roi.total_loc.toLocaleString()} lines` : null,
      featured: false,
    },
  ];

  const timestat = {
    label: 'Avg Session',
    value: isLoading ? null : (roi ? formatSecs(roi.avg_session_duration_secs) : '—'),
    sub: roi ? `${formatSecs(roi.total_active_time_secs)} total` : null,
  };

  return (
    <div className="rounded-xl border border-border bg-card sm:flex sm:divide-x divide-border">
      {/* Cost group: 3 metrics that share a common unit */}
      <div className="flex-[3] flex divide-y sm:divide-y-0 sm:divide-x divide-border flex-col sm:flex-row">
        {costStats.map((s) => (
          <div key={s.label} className="flex-1 px-4 py-4 sm:py-5 flex sm:flex-col items-center sm:items-start justify-between gap-1">
            <p className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground whitespace-nowrap">
              {s.label}
            </p>
            <div className="text-right sm:text-left">
              <p className={s.featured
                ? 'text-4xl font-black tabular-nums leading-none tracking-tighter'
                : 'text-2xl font-bold tabular-nums leading-none tracking-tight'}>
                {s.value === null
                  ? <span className={`inline-block rounded bg-muted animate-shimmer ${s.featured ? 'w-20 h-9' : 'w-14 h-6'}`} />
                  : s.value}
              </p>
              {s.sub && (
                <p className="text-[11px] text-muted-foreground mt-1">{s.sub}</p>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Time metric: distinct category, visually separated */}
      <div className="flex-1 px-4 py-4 sm:py-5 flex sm:flex-col items-center sm:items-start justify-between gap-1 border-t sm:border-t-0">
        <p className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 whitespace-nowrap">
          {timestat.label}
        </p>
        <div className="text-right sm:text-left">
          <p className="text-xl font-bold tabular-nums leading-none text-muted-foreground">
            {timestat.value === null
              ? <span className="inline-block w-12 h-5 rounded bg-muted animate-shimmer" />
              : timestat.value}
          </p>
          {timestat.sub && (
            <p className="text-[11px] text-muted-foreground/70 mt-1">{timestat.sub}</p>
          )}
        </div>
      </div>
    </div>
  );
}
