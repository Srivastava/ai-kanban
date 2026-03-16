'use client';

import { useAnalyticsOverview } from '@/hooks/use-analytics';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function Skeleton() {
  return <span className="animate-pulse bg-muted rounded w-16 h-7 inline-block" />;
}

export function OverviewCards() {
  const { data, isLoading } = useAnalyticsOverview();

  const cached = data
    ? (data.total_cache_creation_tokens ?? 0) + (data.total_cache_read_tokens ?? 0)
    : 0;
  const effectiveTotal = data
    ? data.total_input_tokens + cached + data.total_output_tokens
    : 0;

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {/* Token breakdown — effective total with per-type rows */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-1">
        <p className="text-xs text-muted-foreground uppercase tracking-wider">Token Usage</p>
        <p className="text-2xl font-bold">
          {isLoading ? <Skeleton /> : fmt(effectiveTotal)}
        </p>
        {data && !isLoading ? (
          <div className="text-xs space-y-0.5 mt-1">
            <div className="flex justify-between text-muted-foreground">
              <span>Input</span>
              <span>{fmt(data.total_input_tokens)}</span>
            </div>
            <div className="flex justify-between text-amber-500/80">
              <span>Cached ⚡</span>
              <span>{fmt(cached)}</span>
            </div>
            <div className="flex justify-between text-muted-foreground">
              <span>Output</span>
              <span>{fmt(data.total_output_tokens)}</span>
            </div>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">input · cached · output</p>
        )}
      </div>

      {/* Cost */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-1">
        <p className="text-xs text-muted-foreground uppercase tracking-wider">Estimated Cost</p>
        <p className="text-2xl font-bold">
          {isLoading ? <Skeleton /> : (data ? `$${data.estimated_cost_usd.toFixed(4)}` : '—')}
        </p>
        <p className="text-xs text-muted-foreground">Claude Sonnet pricing</p>
      </div>

      {/* Sessions */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-1">
        <p className="text-xs text-muted-foreground uppercase tracking-wider">Total Sessions</p>
        <p className="text-2xl font-bold">
          {isLoading ? <Skeleton /> : (data ? data.total_sessions.toString() : '—')}
        </p>
        <p className="text-xs text-muted-foreground">{data?.active_sessions_today ?? 0} today</p>
      </div>

      {/* Tasks */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-1">
        <p className="text-xs text-muted-foreground uppercase tracking-wider">Tasks with AI</p>
        <p className="text-2xl font-bold">
          {isLoading ? <Skeleton /> : (data ? data.total_tasks_with_sessions.toString() : '—')}
        </p>
        <p className="text-xs text-muted-foreground">tasks with ≥1 session</p>
      </div>
    </div>
  );
}
