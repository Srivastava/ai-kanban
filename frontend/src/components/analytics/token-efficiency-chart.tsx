'use client';

import { ScatterChart, Scatter, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Label } from 'recharts';
import { useTokenEfficiency } from '@/hooks/use-analytics';
import { formatTokens } from '@/lib/format';
import { TOKEN_COLORS } from '@/lib/chart-colors';

interface Props { taskId?: string | null }

export function TokenEfficiencyChart({ taskId }: Props) {
  const { data: allData = [], isLoading } = useTokenEfficiency();

  const chartData = allData
    .filter((d) => d.lines_written > 0 && d.total_tokens > 0)
    .map((d) => ({
      task_id: d.task_id,
      task_title: d.task_title,
      lines: Math.round(d.lines_written),
      tokens: d.total_tokens,
      tpl: d.tokens_per_line ? Math.round(d.tokens_per_line) : Math.round(d.total_tokens / d.lines_written),
      isSelected: d.task_id === taskId,
    }));

  // Per-task stat card
  if (taskId) {
    const row = chartData.find((d) => d.task_id === taskId) ?? allData.find((d) => d.task_id === taskId);
    const allWithLoc = allData.filter((d) => d.lines_written > 0);
    const sorted = [...allWithLoc]
      .filter((d) => d.tokens_per_line != null && d.tokens_per_line > 0)
      .sort((a, b) => (a.tokens_per_line ?? 0) - (b.tokens_per_line ?? 0));
    const rank = row ? sorted.findIndex((d) => d.task_id === taskId) + 1 : null;
    const tpl = row && 'tokens_per_line' in row && row.tokens_per_line
      ? Math.round(row.tokens_per_line)
      : row && 'tpl' in row ? (row as typeof chartData[0]).tpl : null;

    return (
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <div>
          <h3 className="font-semibold">Token Efficiency</h3>
          <p className="text-xs text-muted-foreground mt-0.5">Tokens per line of code written — lower is more efficient</p>
        </div>
        {isLoading ? (
          <div className="h-24 bg-muted rounded animate-shimmer" />
        ) : tpl == null ? (
          <div className="h-24 flex items-center justify-center">
            <p className="text-muted-foreground text-sm">No LOC data for this task</p>
          </div>
        ) : (
          <div className="flex items-end gap-6">
            <div>
              <p className="text-4xl font-bold tabular-nums text-stage-planning-text">{tpl}</p>
              <p className="text-xs text-muted-foreground mt-1">tokens / line</p>
            </div>
            {rank != null && allWithLoc.length > 1 && (
              <div className="pb-1">
                <p className="text-sm text-muted-foreground">
                  #{rank} most efficient of {allWithLoc.length} tasks
                </p>
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  // Global scatter plot
  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div>
        <h3 className="font-semibold">Token Efficiency</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Each dot is a task — bottom-right is most efficient (more code, fewer tokens)
        </p>
      </div>
      {isLoading ? (
        <div className="h-64 bg-muted rounded animate-shimmer" />
      ) : chartData.length === 0 ? (
        <div className="h-64 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No efficiency data yet — run sessions with LOC tracking enabled</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={280}>
          <ScatterChart margin={{ top: 10, right: 20, left: 10, bottom: 30 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
            <XAxis
              dataKey="lines"
              type="number"
              tickFormatter={formatTokens}
              tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
            >
              <Label value="Lines written" position="insideBottom" offset={-15} style={{ fontSize: 10, fill: 'var(--muted-foreground)' }} />
            </XAxis>
            <YAxis
              dataKey="tokens"
              type="number"
              tickFormatter={formatTokens}
              tick={{ fontSize: 10, fill: 'var(--muted-foreground)' }}
            >
              <Label value="Total tokens" angle={-90} position="insideLeft" offset={15} style={{ fontSize: 10, fill: 'var(--muted-foreground)' }} />
            </YAxis>
            <Tooltip
              cursor={{ strokeDasharray: '3 3' }}
              content={({ active, payload }) => {
                if (!active || !payload?.length) return null;
                const d = payload[0].payload;
                return (
                  <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1 shadow-sm">
                    <p className="font-medium">{d.task_title}</p>
                    <p>{formatTokens(d.tokens)} tokens · {d.lines.toLocaleString()} lines</p>
                    <p className="text-stage-planning-text font-medium">{d.tpl} tokens/line</p>
                  </div>
                );
              }}
            />
            <Scatter
              data={chartData}
              fill={TOKEN_COLORS.input}
              fillOpacity={0.8}
            />
          </ScatterChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
