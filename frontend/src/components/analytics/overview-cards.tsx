'use client';

import { useAnalyticsOverview } from '@/hooks/use-analytics';
import { PRICING } from '@/lib/pricing';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function fmtCost(n: number): string {
  return `$${n.toFixed(2)}`;
}

function Skeleton({ className = 'w-16 h-7' }: { className?: string }) {
  return <span className={`motion-safe:animate-pulse bg-muted rounded inline-block ${className}`} />;
}

export function OverviewCards() {
  const { data, isLoading } = useAnalyticsOverview();

  const cacheCreation = data?.total_cache_creation_tokens ?? 0;
  const cacheRead = data?.total_cache_read_tokens ?? 0;
  const cached = cacheCreation + cacheRead;
  const effectiveTotal = data
    ? data.total_input_tokens + cached + data.total_output_tokens
    : 0;

  const costInput      = data ? (data.total_input_tokens  / 1_000_000) * PRICING.input      : 0;
  const costCacheWrite = data ? (cacheCreation            / 1_000_000) * PRICING.cacheWrite : 0;
  const costCacheRead  = data ? (cacheRead                / 1_000_000) * PRICING.cacheRead  : 0;
  const costOutput     = data ? (data.total_output_tokens / 1_000_000) * PRICING.output     : 0;

  return (
    <div className="space-y-3">
      {/* Featured row: two primary metrics side by side */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {/* Token Usage — featured */}
        <div className="rounded-xl border border-border bg-card p-5">
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-1">Token Usage</p>
          <p className="text-3xl font-bold tabular-nums leading-none mb-3">
            {isLoading ? <Skeleton className="w-24 h-8" /> : fmt(effectiveTotal)}
          </p>
          {data && !isLoading ? (
            <div className="space-y-1 border-t border-border pt-2">
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>New input</span>
                <span className="tabular-nums font-medium">{fmt(data.total_input_tokens)}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-amber-600 dark:text-amber-400">Cache write ⚡</span>
                <span className="tabular-nums font-medium text-amber-600 dark:text-amber-400">{fmt(cacheCreation)}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-amber-500 dark:text-amber-500">Cache read ⚡</span>
                <span className="tabular-nums font-medium text-amber-500">{fmt(cacheRead)}</span>
              </div>
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>Output</span>
                <span className="tabular-nums font-medium">{fmt(data.total_output_tokens)}</span>
              </div>
            </div>
          ) : (
            <p className="text-xs text-muted-foreground border-t border-border pt-2">input · cached · output</p>
          )}
        </div>

        {/* Estimated Cost — featured */}
        <div className="rounded-xl border border-border bg-card p-5">
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-1">Estimated Cost</p>
          <p className="text-3xl font-bold tabular-nums leading-none mb-3">
            {isLoading ? <Skeleton className="w-24 h-8" /> : (data ? fmtCost(data.estimated_cost_usd) : '—')}
          </p>
          {data && !isLoading ? (
            <div className="space-y-1 border-t border-border pt-2">
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>Output</span>
                <span className="tabular-nums font-medium">{fmtCost(costOutput)}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-amber-600 dark:text-amber-400">Cache write</span>
                <span className="tabular-nums font-medium text-amber-600 dark:text-amber-400">{fmtCost(costCacheWrite)}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-amber-500 dark:text-amber-500">Cache read</span>
                <span className="tabular-nums font-medium text-amber-500">{fmtCost(costCacheRead)}</span>
              </div>
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>Input</span>
                <span className="tabular-nums font-medium">{fmtCost(costInput)}</span>
              </div>
            </div>
          ) : (
            <p className="text-xs text-muted-foreground border-t border-border pt-2">Claude Sonnet pricing</p>
          )}
        </div>
      </div>

      {/* Secondary row: compact stats */}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-lg border border-border bg-card px-4 py-3 flex items-center justify-between">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Sessions</p>
            <p className="text-xl font-bold tabular-nums leading-tight mt-0.5">
              {isLoading ? <Skeleton className="w-10 h-6" /> : (data ? data.total_sessions : '—')}
            </p>
          </div>
          {data && !isLoading && data.active_sessions_today > 0 && (
            <span className="text-xs text-green-600 dark:text-green-400 font-medium bg-green-500/10 rounded-full px-2 py-1">
              {data.active_sessions_today} today
            </span>
          )}
        </div>

        <div className="rounded-lg border border-border bg-card px-4 py-3 flex items-center justify-between">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Tasks w/ AI</p>
            <p className="text-xl font-bold tabular-nums leading-tight mt-0.5">
              {isLoading ? <Skeleton className="w-10 h-6" /> : (data ? data.total_tasks_with_sessions : '—')}
            </p>
          </div>
          <span className="text-xs text-muted-foreground">≥1 session</span>
        </div>
      </div>
    </div>
  );
}
