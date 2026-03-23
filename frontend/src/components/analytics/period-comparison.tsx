'use client';

import { usePeriodComparison } from '@/hooks/use-analytics';
import type { PeriodChange } from '@/types/analytics';
import { TrendingUp, TrendingDown, Minus } from 'lucide-react';
import { cn } from '@/lib/utils';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

function Delta({ pct, inverse = false }: { pct: number | null; inverse?: boolean }) {
  if (pct === null) return <span className="text-[10px] text-muted-foreground">—</span>;
  const abs = Math.abs(pct);
  // For cost: up is bad (inverse=false means up=green), for "savings" inverse=true
  const isUp = pct > 0;
  const isGood = inverse ? !isUp : isUp;
  // For cost metrics, going up is bad (red), going down is good (green)
  // Actually user just wants to see change direction — neutral color, show arrow
  const color = Math.abs(pct) < 1 ? 'text-muted-foreground' :
    isUp ? 'text-red-500 dark:text-red-400' : 'text-green-600 dark:text-green-400';
  const Icon = Math.abs(pct) < 1 ? Minus : isUp ? TrendingUp : TrendingDown;
  return (
    <span className={cn('inline-flex items-center gap-0.5 text-[10px] font-medium', color)}>
      <Icon className="h-2.5 w-2.5" />
      {abs < 1 ? '<1' : abs.toFixed(0)}%
    </span>
  );
}

interface PeriodCardProps {
  label: string;
  change: PeriodChange;
}

function PeriodCard({ label, change }: PeriodCardProps) {
  return (
    <div className="rounded-lg border border-border bg-card/60 px-3 py-2.5 space-y-2">
      <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">{label}</p>
      <div className="space-y-1.5">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">Cost</span>
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] font-semibold">${change.current.cost_usd.toFixed(2)}</span>
            <Delta pct={change.cost_pct} />
          </div>
        </div>
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">Tokens</span>
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] font-semibold">{fmt(change.current.tokens)}</span>
            <Delta pct={change.tokens_pct} />
          </div>
        </div>
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">Sessions</span>
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] font-semibold">{change.current.sessions}</span>
            <Delta pct={change.sessions_pct} />
          </div>
        </div>
      </div>
    </div>
  );
}

export function PeriodComparison() {
  const { data, isLoading } = usePeriodComparison();

  if (isLoading) {
    return (
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Period over Period</h3>
        <div className="grid grid-cols-3 gap-3">
          {[0,1,2].map(i => (
            <div key={i} className="h-24 animate-pulse bg-muted rounded-lg" />
          ))}
        </div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Period over Period</h3>
        <p className="text-[10px] text-muted-foreground">vs previous period · arrows show change</p>
      </div>
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
        <PeriodCard label="Day over Day" change={data.day} />
        <PeriodCard label="Week over Week" change={data.week} />
        <PeriodCard label="Month over Month" change={data.month} />
      </div>
    </div>
  );
}
