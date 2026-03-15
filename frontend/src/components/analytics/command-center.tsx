'use client';

import { useAnalyticsOverview, useBurnRate, useUsageWindows, usePlanTier } from '@/hooks/use-analytics';
import { RateLimitGauge } from './rate-limit-gauge';
import { ContextWindowGauges } from './context-window-gauge';

function Stat({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="space-y-0.5">
      <p className="text-xs text-muted-foreground uppercase tracking-wider">{label}</p>
      <p className="text-2xl font-bold tabular-nums">{value}</p>
      {sub && <p className="text-xs text-muted-foreground">{sub}</p>}
    </div>
  );
}

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function CommandCenter() {
  const { data: overview } = useAnalyticsOverview();
  const { data: burn } = useBurnRate();
  const { data: windows } = useUsageWindows();
  const { data: plan } = usePlanTier();

  const limit5hr = plan?.limit_5hr ?? 19_000;
  const limitWeek = plan?.limit_week ?? 1_000_000;

  const burnLabel = burn
    ? `${formatTokens(Math.round(burn.tokens_per_minute * 60))}/hr — ${
        burn.tokens_per_minute > 0 && windows
          ? `limit in ~${Math.round((limit5hr - (windows.tokens_5hr ?? 0)) / burn.tokens_per_minute / 60)}h`
          : 'at limit pace'
      }`
    : null;

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
        <Stat
          label="Total Cost"
          value={overview ? `$${overview.estimated_cost_usd.toFixed(2)}` : '—'}
        />
        <Stat
          label="Total Tokens"
          value={overview ? formatTokens(overview.total_input_tokens + overview.total_output_tokens) : '—'}
          sub={overview ? `${formatTokens(overview.total_input_tokens)} in / ${formatTokens(overview.total_output_tokens)} out` : ''}
        />
        <Stat
          label="Sessions"
          value={overview ? String(overview.total_sessions) : '—'}
          sub={overview ? `${overview.active_sessions_today} today` : ''}
        />
      </div>
    </div>
  );
}
