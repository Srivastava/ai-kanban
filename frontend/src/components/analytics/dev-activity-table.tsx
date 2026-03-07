'use client';

import { useDevActivity } from '@/hooks/use-analytics';
import type { DevActivityRow } from '@/types/analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function DevActivityTable() {
  const { data, isLoading } = useDevActivity();

  if (isLoading) {
    return <div className="h-32 animate-pulse rounded-lg bg-muted" />;
  }

  if (!data || data.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
        No dev activity yet. Data appears here once Claude Code sessions complete.
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted/50">
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Task</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Sessions</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Lines +/-</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Input / Output</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Cache Read</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Cost (OTel)</th>
          </tr>
        </thead>
        <tbody>
          {data.map((row: DevActivityRow) => (
            <tr key={row.task_id} className="border-b border-border last:border-0 hover:bg-muted/30 transition-colors">
              <td className="px-4 py-3 font-medium truncate max-w-[200px]" title={row.task_title}>
                {row.task_title}
              </td>
              <td className="px-4 py-3 text-right tabular-nums text-muted-foreground">
                {row.session_count}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                {row.lines_added > 0 || row.lines_deleted > 0 ? (
                  <>
                    <span className="text-green-600">+{Math.round(row.lines_added)}</span>
                    {' / '}
                    <span className="text-red-500">-{Math.round(row.lines_deleted)}</span>
                  </>
                ) : (
                  <span className="text-muted-foreground">—</span>
                )}
              </td>
              <td className="px-4 py-3 text-right tabular-nums">
                <span className="text-blue-500">{formatTokens(row.input_tokens)}</span>
                {' / '}
                <span className="text-violet-500">{formatTokens(row.output_tokens)}</span>
              </td>
              <td className="px-4 py-3 text-right tabular-nums text-muted-foreground">
                {formatTokens(row.cache_read_tokens)}
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
