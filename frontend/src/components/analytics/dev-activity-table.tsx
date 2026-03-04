'use client';

import { useDevActivity } from '@/hooks/use-analytics';
import type { DevActivityRow } from '@/types/analytics';

function formatTime(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  return `${(secs / 3600).toFixed(1)}h`;
}

export function DevActivityTable() {
  const { data, isLoading } = useDevActivity();

  if (isLoading) {
    return <div className="h-32 animate-pulse rounded-lg bg-muted" />;
  }

  if (!data || data.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
        No dev activity yet. Data appears here once Claude Code sessions emit OTel metrics.
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted/50">
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Task</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Lines +/-</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Commits</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">PRs</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Active Time</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Cost</th>
          </tr>
        </thead>
        <tbody>
          {data.map((row: DevActivityRow) => (
            <tr key={`${row.task_id}-${row.session_id}`} className="border-b border-border last:border-0 hover:bg-muted/30 transition-colors">
              <td className="px-4 py-3 font-medium truncate max-w-[200px]" title={row.task_title}>
                {row.task_title}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                <span className="text-green-600">+{Math.round(row.lines_added)}</span>
                {' / '}
                <span className="text-red-500">-{Math.round(row.lines_deleted)}</span>
              </td>
              <td className="px-4 py-3 text-right tabular-nums">{Math.round(row.commits)}</td>
              <td className="px-4 py-3 text-right tabular-nums">{Math.round(row.pull_requests)}</td>
              <td className="px-4 py-3 text-right tabular-nums text-muted-foreground">
                {formatTime(row.active_time_secs)}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                {row.cost_usd > 0 ? `$${row.cost_usd.toFixed(4)}` : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
