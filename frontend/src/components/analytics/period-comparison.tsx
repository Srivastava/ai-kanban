'use client';

import { usePeriodComparison } from '@/hooks/use-analytics';
import { TrendingUp, TrendingDown, Minus } from 'lucide-react';
import { cn } from '@/lib/utils';

function Delta({ pct, label, currentVal }: { pct: number | null; label: string; currentVal?: number }) {
  if (pct === null) {
    // No prior period — show "New" if there is current data
    if (!currentVal) return null;
    return (
      <span
        className="inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 text-[10px] font-semibold text-blue-400 bg-blue-500/10"
        title={`${label}: no prior period data`}
      >
        <TrendingUp className="h-2.5 w-2.5 shrink-0" />
        {label} New
      </span>
    );
  }
  const abs = Math.abs(pct);
  const isUp = pct > 0;
  const neutral = abs < 1;
  const color = neutral
    ? 'text-muted-foreground'
    : isUp
    ? 'text-red-500 dark:text-red-400'
    : 'text-green-600 dark:text-green-400';
  const bg = neutral
    ? 'bg-muted/60'
    : isUp
    ? 'bg-red-500/10'
    : 'bg-green-500/10';
  const Icon = neutral ? Minus : isUp ? TrendingUp : TrendingDown;
  return (
    <span
      className={cn(
        'inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 text-[10px] font-semibold',
        color, bg
      )}
      title={`${label}: ${isUp ? '+' : ''}${pct.toFixed(1)}% vs prior period`}
    >
      <Icon className="h-2.5 w-2.5 shrink-0" />
      {label} {neutral ? '<1%' : `${abs.toFixed(0)}%`}
    </span>
  );
}

/** Drop these badges into any chart header to show WoW / MoM for tokens. */
export function TokenTrendBadges() {
  const { data } = usePeriodComparison();
  if (!data) return null;
  return (
    <div className="flex items-center gap-1 flex-wrap">
      <Delta pct={data.week.tokens_pct} label="WoW" currentVal={data.week.current.tokens} />
      <Delta pct={data.month.tokens_pct} label="MoM" currentVal={data.month.current.tokens} />
    </div>
  );
}

/** Drop these badges into any chart header to show WoW / MoM for cost. */
export function CostTrendBadges() {
  const { data } = usePeriodComparison();
  if (!data) return null;
  return (
    <div className="flex items-center gap-1 flex-wrap">
      <Delta pct={data.week.cost_pct} label="WoW" currentVal={data.week.current.cost_usd} />
      <Delta pct={data.month.cost_pct} label="MoM" currentVal={data.month.current.cost_usd} />
    </div>
  );
}

/** Drop these badges into any chart header to show WoW / MoM for sessions. */
export function SessionTrendBadges() {
  const { data } = usePeriodComparison();
  if (!data) return null;
  return (
    <div className="flex items-center gap-1 flex-wrap">
      <Delta pct={data.week.sessions_pct} label="WoW" currentVal={data.week.current.sessions} />
      <Delta pct={data.month.sessions_pct} label="MoM" currentVal={data.month.current.sessions} />
    </div>
  );
}
