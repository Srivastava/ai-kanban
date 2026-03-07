'use client';

import { useUsageWindows } from '@/hooks/use-analytics';
import { cn } from '@/lib/utils';
import { RefreshCw } from 'lucide-react';

const TZ = 'America/Los_Angeles';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

function formatReset(iso: string | null): { label: string; countdown: string } {
  if (!iso) return { label: '—', countdown: 'no usage' };
  const d = new Date(iso);
  const label = new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
    timeZoneName: 'short',
  }).format(d);

  const msLeft = d.getTime() - Date.now();
  if (msLeft <= 0) return { label, countdown: 'resetting…' };
  const h = Math.floor(msLeft / 3_600_000);
  const m = Math.floor((msLeft % 3_600_000) / 60_000);
  const countdown = h > 0 ? `${h}h ${m}m` : `${m}m`;
  return { label, countdown };
}

interface GaugeProps {
  used: number;
  limit: number;
  colorClass: string;
}

function UsageGauge({ used, limit, colorClass }: GaugeProps) {
  if (limit <= 0) return null;
  const pct = Math.min(100, (used / limit) * 100);
  const warn = pct >= 80;
  return (
    <div className="mt-2 space-y-1">
      <div className="h-1.5 w-full rounded-full bg-muted overflow-hidden">
        <div
          className={cn('h-full rounded-full transition-all', warn ? 'bg-red-500' : colorClass)}
          style={{ width: `${pct}%` }}
        />
      </div>
      <p className="text-[10px] text-muted-foreground">
        {formatTokens(used)} / {formatTokens(limit)}
        &nbsp;·&nbsp;{formatTokens(Math.max(0, limit - used))} remaining
      </p>
    </div>
  );
}

export function UsageWindowsCard() {
  const { data, isLoading, isFetching, refetch, dataUpdatedAt } = useUsageWindows();

  const lastUpdated = dataUpdatedAt
    ? new Intl.DateTimeFormat('en-US', {
        timeZone: TZ,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
      }).format(new Date(dataUpdatedAt))
    : null;

  const reset5hr = formatReset(data?.reset_5hr ?? null);
  const resetWeek = formatReset(data?.reset_week ?? null);

  const skeleton = (
    <span className="animate-pulse bg-muted rounded w-16 h-6 inline-block" />
  );

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-start justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold">Claude Rate Limits</h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            Tokens consumed in the current rolling windows
            {lastUpdated && (
              <span className="ml-2 opacity-60">· updated {lastUpdated}</span>
            )}
          </p>
        </div>
        <button
          onClick={() => refetch()}
          disabled={isFetching}
          title="Refresh usage data"
          className="shrink-0 p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-muted/60 transition-colors disabled:opacity-40"
        >
          <RefreshCw className={cn('h-3.5 w-3.5', isFetching && 'animate-spin')} />
        </button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* 5-hour window */}
        <div className="space-y-1">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">5-hr tokens used</p>
          <p className="text-2xl font-bold">
            {isLoading ? skeleton : formatTokens(data?.tokens_5hr ?? 0)}
          </p>
          {data && (
            <UsageGauge
              used={data.tokens_5hr}
              limit={data.limit_5hr}
              colorClass="bg-blue-500"
            />
          )}
        </div>

        {/* Weekly window */}
        <div className="space-y-1">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">Weekly tokens used</p>
          <p className="text-2xl font-bold">
            {isLoading ? skeleton : formatTokens(data?.tokens_week ?? 0)}
          </p>
          {data && (
            <UsageGauge
              used={data.tokens_week}
              limit={data.limit_week}
              colorClass="bg-violet-500"
            />
          )}
        </div>

        {/* 5-hour reset */}
        <div className="space-y-1 border-t border-border/50 pt-3">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">5-hr reset</p>
          {isLoading ? (
            skeleton
          ) : (
            <>
              <p className="text-base font-semibold">{reset5hr.countdown}</p>
              <p className="text-[11px] text-muted-foreground font-mono">{reset5hr.label}</p>
            </>
          )}
        </div>

        {/* Weekly reset */}
        <div className="space-y-1 border-t border-border/50 pt-3">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">Weekly reset</p>
          {isLoading ? (
            skeleton
          ) : (
            <>
              <p className="text-base font-semibold">{resetWeek.countdown}</p>
              <p className="text-[11px] text-muted-foreground font-mono">{resetWeek.label}</p>
            </>
          )}
        </div>
      </div>

      {data && data.limit_5hr === 0 && data.limit_week === 0 && (
        <p className="text-[11px] text-muted-foreground border-t border-border/50 pt-3">
          Set <code className="font-mono bg-muted px-1 rounded">CLAUDE_5HR_TOKEN_LIMIT</code> and{' '}
          <code className="font-mono bg-muted px-1 rounded">CLAUDE_WEEKLY_TOKEN_LIMIT</code> env vars
          to show remaining tokens.
        </p>
      )}
    </div>
  );
}
