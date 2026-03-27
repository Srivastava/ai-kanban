'use client';

import { useCostByTask } from '@/hooks/use-analytics';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

interface Props { taskId?: string | null }

export function CostBreakdownTable({ taskId }: Props) {
  const { data: allData = [], isLoading } = useCostByTask();
  const data = taskId ? allData.filter((r) => r.task_id === taskId) : allData;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Top Tasks by Cost</h3>
      {isLoading ? (
        <div className="h-48 bg-muted rounded animate-shimmer" />
      ) : data.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No cost data yet</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-xs text-muted-foreground uppercase tracking-wider">
                <th className="pb-2 pr-4 font-medium">Task</th>
                <th className="pb-2 pr-4 font-medium text-right">Input</th>
                <th className="pb-2 pr-4 font-medium text-right text-amber-500/80">Cached</th>
                <th className="pb-2 pr-4 font-medium text-right">Output</th>
                <th className="pb-2 font-medium text-right">Cost</th>
              </tr>
            </thead>
            <tbody>
              {data.map((row) => {
                const cached = (row.cache_creation_tokens ?? 0) + (row.cache_read_tokens ?? 0);
                return (
                  <tr key={row.task_id} className="border-b border-border/50 last:border-0">
                    <td className="py-2 pr-4 truncate max-w-[200px]" title={row.task_title}>
                      {row.task_title}
                    </td>
                    <td className="py-2 pr-4 text-right text-muted-foreground">
                      {fmt(row.input_tokens)}
                    </td>
                    <td className="py-2 pr-4 text-right text-amber-500/80">
                      {fmt(cached)}
                    </td>
                    <td className="py-2 pr-4 text-right text-muted-foreground">
                      {fmt(row.output_tokens)}
                    </td>
                    <td className="py-2 text-right font-medium tabular-nums">
                      ${row.cost_usd.toFixed(6)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
