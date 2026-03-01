'use client';

import { useSessionSummary, useBurnRate } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

const skeleton = <span className="animate-pulse bg-muted rounded w-16 h-7 inline-block" />;

export function SessionIntelligenceCard() {
  const { data: summary, isLoading: summaryLoading } = useSessionSummary();
  const { data: burnRate, isLoading: burnLoading } = useBurnRate();
  const isLoading = summaryLoading || burnLoading;

  const cards = [
    { label: 'Total Sessions', value: summary ? summary.total_sessions.toString() : '—', sub: 'lifetime sessions' },
    { label: 'Avg Tokens / Session', value: summary ? formatTokens(Math.round(summary.avg_tokens_per_session)) : '—', sub: 'input + output combined' },
    { label: 'Max Tokens / Session', value: summary ? formatTokens(summary.max_tokens_per_session) : '—', sub: 'single session peak' },
    { label: 'Total Cost', value: summary ? `$${summary.total_cost_usd.toFixed(4)}` : '—', sub: 'Claude Sonnet pricing' },
    { label: 'Last-Hour Tokens', value: burnRate ? formatTokens(Math.round(burnRate.tokens_last_hour)) : '—', sub: 'rolling 60-minute window' },
    { label: 'Burn Rate', value: burnRate ? `${Math.round(burnRate.tokens_per_minute).toLocaleString()} tokens/min` : '—', sub: 'average over last hour' },
  ];

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Agent Intelligence</h3>
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-3">
        {cards.map((card) => (
          <div key={card.label} className="space-y-1">
            <p className="text-xs text-muted-foreground uppercase tracking-wider">{card.label}</p>
            <p className="text-xl font-bold">{isLoading ? skeleton : card.value}</p>
            <p className="text-xs text-muted-foreground">{card.sub}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
