'use client';

import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAnalyticsOverview, useBurnRate, useUsageWindows, usePlanTier } from '@/hooks/use-analytics';
import { PRICING } from '@/lib/pricing';
import { RateLimitGauge } from './rate-limit-gauge';
import { ContextWindowGauges } from './context-window-gauge';
import { useWebSocket } from '@/contexts/websocket-context';

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function fmtCost(n: number) { return `$${n.toFixed(2)}`; }

function BurnRateSparkline({ value }: { value: number }) {
  const bufRef = useRef<number[]>(Array(10).fill(0));
  const [points, setPoints] = useState<number[]>(Array(10).fill(0));

  useEffect(() => {
    if (value === 0) return;
    bufRef.current = [...bufRef.current.slice(1), value];
    setPoints([...bufRef.current]);
  }, [value]);

  const max = Math.max(...points, 0.001);
  const W = 60, H = 20;
  const pts = points.map((v, i) =>
    `${(i / 9) * W},${H - (v / max) * H}`
  ).join(' ');

  return (
    <svg width={W} height={H} className="inline-block opacity-70" aria-hidden="true">
      <polyline points={pts} fill="none" style={{ stroke: 'var(--muted-foreground)' }} strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  );
}

export function CommandCenter() {
  const { data: overview } = useAnalyticsOverview();
  const { data: burn } = useBurnRate();
  const { data: windows } = useUsageWindows();
  const { data: plan } = usePlanTier();

  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();
  const [hasRunning, setHasRunning] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Detect running sessions on mount
  useEffect(() => {
    fetch('/api/sessions')
      .then(r => r.json())
      .then((data: { active_count?: number }) => {
        if ((data.active_count ?? 0) > 0) setHasRunning(true);
      })
      .catch(() => {});
  }, []);

  // Track running sessions via WS
  useEffect(() => {
    const onStarted = () => setHasRunning(true);

    const onEnded = () => {
      // Immediately fetch final cost before potentially clearing live indicator
      queryClient.invalidateQueries({ queryKey: ['analytics', 'overview'] });
      // Check if any others still running before clearing live state
      fetch('/api/sessions')
        .then(r => r.json())
        .then((data: { active_count?: number }) => {
          setHasRunning((data.active_count ?? 0) > 0);
        })
        .catch(() => setHasRunning(false));
    };

    const unsubStarted   = subscribe('session_started',   onStarted);
    const unsubCompleted = subscribe('session_completed',  onEnded);
    const unsubFailed    = subscribe('session_failed',     onEnded);
    const unsubStopped   = subscribe('session_stopped',    onEnded);

    return () => {
      unsubStarted();
      unsubCompleted();
      unsubFailed();
      unsubStopped();
    };
  }, [subscribe, queryClient]);

  // Poll overview faster when running (10s), respecting rate limits
  useEffect(() => {
    if (intervalRef.current) clearInterval(intervalRef.current);
    if (hasRunning) {
      intervalRef.current = setInterval(() => {
        queryClient.invalidateQueries({ queryKey: ['analytics', 'overview'] });
      }, 10_000);
    }
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [hasRunning, queryClient]);

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

  const costInput      = overview ? (overview.total_input_tokens / 1_000_000) * PRICING.input      : 0;
  const costCacheWrite = overview ? (cacheCreation               / 1_000_000) * PRICING.cacheWrite : 0;
  const costCacheRead  = overview ? (cacheRead                   / 1_000_000) * PRICING.cacheRead  : 0;
  const costOutput     = overview ? (overview.total_output_tokens/ 1_000_000) * PRICING.output      : 0;

  return (
    <div className="rounded-xl border border-border bg-card p-5 sm:p-6 space-y-6">
      {/* Active session banner */}
      {hasRunning && (
        <div className="flex items-center gap-2 text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg px-3 py-2">
          <span className="h-2 w-2 rounded-full bg-amber-500 motion-safe:animate-breathe shrink-0" />
          <span>Session running — cost updating every 10s</span>
        </div>
      )}

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
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>Burn rate: <span className="font-medium text-foreground">{burnLabel}</span></span>
          <BurnRateSparkline value={burn?.tokens_per_minute ?? 0} />
        </div>
      )}

      {/* Context gauges */}
      <div className="space-y-2">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Active session context
        </p>
        <ContextWindowGauges />
      </div>

      {/* Headline stats — intentional hierarchy: cost > tokens > sessions */}
      <div className="grid grid-cols-3 gap-4 sm:gap-6 pt-4 border-t border-border items-start">
        {/* Cost — primary: hero number */}
        <div className="space-y-1 col-span-3 sm:col-span-1">
          <div className="flex items-center gap-2">
            <p className="text-[10px] font-bold uppercase tracking-widest text-amber-600 dark:text-amber-500">Total Cost</p>
            {hasRunning && (
              <span className="flex items-center gap-1 text-[10px] text-amber-500 font-semibold">
                <span className="h-1.5 w-1.5 rounded-full bg-amber-500 motion-safe:animate-breathe" />
                live
              </span>
            )}
          </div>
          <p className="text-5xl font-black tabular-nums leading-none tracking-tighter">
            {overview ? fmtCost(overview.estimated_cost_usd) : '—'}
          </p>
          {overview && (
            <div className="text-xs space-y-0.5 mt-2 border-t border-border/50 pt-2">
              <div className="flex justify-between text-muted-foreground"><span>Output</span><span className="tabular-nums">{fmtCost(costOutput)}</span></div>
              <div className="flex justify-between text-amber-500/80"><span>Cache write</span><span className="tabular-nums">{fmtCost(costCacheWrite)}</span></div>
              <div className="flex justify-between text-amber-400/70"><span>Cache read</span><span className="tabular-nums">{fmtCost(costCacheRead)}</span></div>
              <div className="flex justify-between text-muted-foreground"><span>Input</span><span className="tabular-nums">{fmtCost(costInput)}</span></div>
            </div>
          )}
        </div>

        {/* Tokens — secondary */}
        <div className="space-y-1">
          <p className="text-[10px] font-bold uppercase tracking-widest text-primary/80">Total Tokens</p>
          <p className="text-3xl font-black tabular-nums leading-none tracking-tighter">
            {overview ? fmt(effectiveTotal) : '—'}
          </p>
          {overview && (
            <div className="text-xs space-y-0.5 mt-2 border-t border-border/50 pt-2">
              <div className="flex justify-between text-muted-foreground"><span>New input</span><span className="tabular-nums">{fmt(overview.total_input_tokens)}</span></div>
              <div className="flex justify-between text-amber-500/80"><span>Cache write</span><span className="tabular-nums">{fmt(cacheCreation)}</span></div>
              <div className="flex justify-between text-amber-400/70"><span>Cache read</span><span className="tabular-nums">{fmt(cacheRead)}</span></div>
              <div className="flex justify-between text-muted-foreground"><span>Output</span><span className="tabular-nums">{fmt(overview.total_output_tokens)}</span></div>
            </div>
          )}
        </div>

        {/* Sessions — tertiary */}
        <div className="space-y-1">
          <p className="text-[10px] font-bold uppercase tracking-widest text-emerald-600 dark:text-emerald-500">Sessions</p>
          <p className="text-2xl font-black tabular-nums leading-none tracking-tighter text-muted-foreground/80">
            {overview ? String(overview.total_sessions) : '—'}
          </p>
          {overview && (
            <p className="text-xs text-muted-foreground mt-2 border-t border-border/50 pt-2">
              {overview.active_sessions_today > 0
                ? <span className="text-emerald-500 font-semibold">{overview.active_sessions_today} active now</span>
                : 'none active'}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
