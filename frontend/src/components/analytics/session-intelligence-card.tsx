'use client';

import { useSessionSummary, useBurnRate } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

const skeleton = <span className="bg-muted rounded w-16 h-7 inline-block animate-shimmer" />;

export function SessionIntelligenceCard() {
  const { data: summary, isLoading: summaryLoading } = useSessionSummary();
  const { data: burnRate, isLoading: burnLoading } = useBurnRate();
  const isLoading = summaryLoading || burnLoading;

  const hero = [
    { label: 'Total Sessions', value: summary ? summary.total_sessions.toString() : '—', sub: 'lifetime' },
    { label: 'Total Cost', value: summary ? `$${summary.total_cost_usd.toFixed(4)}` : '—', sub: 'Claude Sonnet pricing' },
  ];

  const secondary = [
    { label: 'Avg Tokens / Session', value: summary ? formatTokens(Math.round(summary.avg_tokens_per_session)) : '—', sub: 'input + output' },
    { label: 'Max Tokens / Session', value: summary ? formatTokens(summary.max_tokens_per_session) : '—', sub: 'single session peak' },
    { label: 'Last-Hour Tokens', value: burnRate ? formatTokens(Math.round(burnRate.tokens_last_hour)) : '—', sub: 'rolling 60 min' },
    { label: 'Burn Rate', value: burnRate ? `${Math.round(burnRate.tokens_per_minute).toLocaleString()} tokens/min` : '—', sub: 'avg over last hour' },
  ];

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Agent Intelligence</h3>

      {/* Hero row: the two metrics that matter most */}
      <div className="grid grid-cols-2 gap-4 pb-4 border-b border-border">
        {hero.map((card) => (
          <div key={card.label} className="space-y-0.5">
            <p className="text-xs text-muted-foreground uppercase tracking-wider">{card.label}</p>
            <p className="text-2xl font-bold tabular-nums leading-none">{isLoading ? skeleton : card.value}</p>
            <p className="text-xs text-muted-foreground">{card.sub}</p>
          </div>
        ))}
      </div>

      {/* Supporting metrics: smaller, clearly secondary */}
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        {secondary.map((card) => (
          <div key={card.label} className="space-y-0.5">
            <p className="text-[11px] text-muted-foreground uppercase tracking-wider">{card.label}</p>
            <p className="text-base font-semibold tabular-nums leading-none text-muted-foreground/90">
              {isLoading ? <span className="inline-block w-12 h-4 rounded bg-muted animate-shimmer" /> : card.value}
            </p>
            <p className="text-[11px] text-muted-foreground/70">{card.sub}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
