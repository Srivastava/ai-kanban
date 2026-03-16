'use client';

import { useAnalyticsOverview } from '@/hooks/use-analytics';

// Sonnet 3.5/3.7 pricing per 1M tokens
const INPUT_PRICE = 3.0;
const OUTPUT_PRICE = 15.0;
const CACHE_WRITE_PRICE = 3.75;
const CACHE_READ_PRICE = 0.30;

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function fmtCost(n: number): string {
  return `$${n.toFixed(2)}`;
}

function Skeleton() {
  return <span className="animate-pulse bg-muted rounded w-16 h-7 inline-block" />;
}

export function OverviewCards() {
  const { data, isLoading } = useAnalyticsOverview();

  const cacheCreation = data?.total_cache_creation_tokens ?? 0;
  const cacheRead = data?.total_cache_read_tokens ?? 0;
  const cached = cacheCreation + cacheRead;
  const effectiveTotal = data
    ? data.total_input_tokens + cached + data.total_output_tokens
    : 0;

  // Cost breakdown by token type
  const costInput        = data ? (data.total_input_tokens   / 1_000_000) * INPUT_PRICE        : 0;
  const costCacheWrite   = data ? (cacheCreation             / 1_000_000) * CACHE_WRITE_PRICE  : 0;
  const costCacheRead    = data ? (cacheRead                 / 1_000_000) * CACHE_READ_PRICE   : 0;
  const costOutput       = data ? (data.total_output_tokens  / 1_000_000) * OUTPUT_PRICE       : 0;

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
              <span>New input</span>
              <span>{fmt(data.total_input_tokens)}</span>
            </div>
            <div className="flex justify-between text-amber-500/80">
              <span>Cache write ⚡</span>
              <span>{fmt(cacheCreation)}</span>
            </div>
            <div className="flex justify-between text-amber-400/70">
              <span>Cache read ⚡</span>
              <span>{fmt(cacheRead)}</span>
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

      {/* Cost with breakdown */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-1">
        <p className="text-xs text-muted-foreground uppercase tracking-wider">Estimated Cost</p>
        <p className="text-2xl font-bold">
          {isLoading ? <Skeleton /> : (data ? fmtCost(data.estimated_cost_usd) : '—')}
        </p>
        {data && !isLoading ? (
          <div className="text-xs space-y-0.5 mt-1">
            <div className="flex justify-between text-muted-foreground">
              <span>Output</span>
              <span>{fmtCost(costOutput)}</span>
            </div>
            <div className="flex justify-between text-amber-500/80">
              <span>Cache write</span>
              <span>{fmtCost(costCacheWrite)}</span>
            </div>
            <div className="flex justify-between text-amber-400/70">
              <span>Cache read</span>
              <span>{fmtCost(costCacheRead)}</span>
            </div>
            <div className="flex justify-between text-muted-foreground">
              <span>Input</span>
              <span>{fmtCost(costInput)}</span>
            </div>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">Claude Sonnet pricing</p>
        )}
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
