'use client';

import { useAnalyticsOverview, useBurnRate, useUsageWindows, usePlanTier } from '@/hooks/use-analytics';
import { RateLimitGauge } from './rate-limit-gauge';
import { ContextWindowGauges } from './context-window-gauge';

// Sonnet pricing per 1M tokens
const INPUT_PRICE = 3.0;
const OUTPUT_PRICE = 15.0;
const CACHE_WRITE_PRICE = 3.75;
const CACHE_READ_PRICE = 0.30;

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function fmtCost(n: number) { return `$${n.toFixed(2)}`; }

export function CommandCenter() {
  const { data: overview } = useAnalyticsOverview();
  const { data: burn } = useBurnRate();
  const { data: windows } = useUsageWindows();
  const { data: plan } = usePlanTier();

  const limit5hr = plan?.limit_5hr ?? 350_000;
  const limitWeek = plan?.limit_week ?? 3_500_000;

  const burnLabel = burn
    ? `${fmt(Math.round(burn.tokens_per_minute * 60))}/hr — ${
        burn.tokens_per_minute > 0 && windows
          ? `limit in ~${Math.round((limit5hr - (windows.tokens_5hr ?? 0)) / burn.tokens_per_minute / 60)}h`
          : 'at limit pace'
      }`
    : null;

  const cacheCreation = overview?.total_cache_creation_tokens ?? 0;
  const cacheRead = overview?.total_cache_read_tokens ?? 0;
  const effectiveTotal = overview
    ? overview.total_input_tokens + cacheCreation + cacheRead + overview.total_output_tokens
    : 0;

  const costInput      = overview ? (overview.total_input_tokens / 1_000_000) * INPUT_PRICE      : 0;
  const costCacheWrite = overview ? (cacheCreation               / 1_000_000) * CACHE_WRITE_PRICE : 0;
  const costCacheRead  = overview ? (cacheRead                   / 1_000_000) * CACHE_READ_PRICE  : 0;
  const costOutput     = overview ? (overview.total_output_tokens/ 1_000_000) * OUTPUT_PRICE      : 0;

  return (
    <div className="rounded-xl border border-border bg-gradient-to-br from-card to-card/60 p-5 sm:p-6 space-y-6">
      {/* Rate limit gauges */}
      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
        <RateLimitGauge
          label={`5-hour window (${plan?.tier ?? 'pro'} plan)`}
          used={windows?.tokens_5hr ?? 0}
          limit={limit5hr}
          resetAt={windows?.reset_5hr ?? null}
        />
        <RateLimitGauge
          label="Weekly window"
          used={windows?.tokens_week ?? 0}
          limit={limitWeek}
          resetAt={windows?.reset_week ?? null}
          showDate
        />
      </div>

      {/* Burn rate */}
      {burnLabel && (
        <p className="text-xs text-muted-foreground">
          Burn rate: <span className="font-medium text-foreground">{burnLabel}</span>
        </p>
      )}

      {/* Context gauges */}
      <div className="space-y-2">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Active session context
        </p>
        <ContextWindowGauges />
      </div>

      {/* Headline stats */}
      <div className="grid grid-cols-3 gap-4 pt-2 border-t border-border">
        {/* Cost with breakdown */}
        <div className="space-y-0.5">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">Total Cost</p>
          <p className="text-2xl font-bold tabular-nums">{overview ? fmtCost(overview.estimated_cost_usd) : '—'}</p>
          {overview && (
            <div className="text-xs space-y-0.5 mt-1">
              <div className="flex justify-between text-muted-foreground"><span>Output</span><span>{fmtCost(costOutput)}</span></div>
              <div className="flex justify-between text-amber-500/80"><span>Cache write</span><span>{fmtCost(costCacheWrite)}</span></div>
              <div className="flex justify-between text-amber-400/70"><span>Cache read</span><span>{fmtCost(costCacheRead)}</span></div>
              <div className="flex justify-between text-muted-foreground"><span>Input</span><span>{fmtCost(costInput)}</span></div>
            </div>
          )}
        </div>

        {/* Tokens with breakdown */}
        <div className="space-y-0.5">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">Total Tokens</p>
          <p className="text-2xl font-bold tabular-nums">{overview ? fmt(effectiveTotal) : '—'}</p>
          {overview && (
            <div className="text-xs space-y-0.5 mt-1">
              <div className="flex justify-between text-muted-foreground"><span>New input</span><span>{fmt(overview.total_input_tokens)}</span></div>
              <div className="flex justify-between text-amber-500/80"><span>Cache write</span><span>{fmt(cacheCreation)}</span></div>
              <div className="flex justify-between text-amber-400/70"><span>Cache read</span><span>{fmt(cacheRead)}</span></div>
              <div className="flex justify-between text-muted-foreground"><span>Output</span><span>{fmt(overview.total_output_tokens)}</span></div>
            </div>
          )}
        </div>

        {/* Sessions */}
        <div className="space-y-0.5">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">Sessions</p>
          <p className="text-2xl font-bold tabular-nums">{overview ? String(overview.total_sessions) : '—'}</p>
          {overview && <p className="text-xs text-muted-foreground">{overview.active_sessions_today} today</p>}
        </div>
      </div>
    </div>
  );
}
