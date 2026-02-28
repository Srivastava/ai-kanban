'use client';

import { useAnalyticsOverview } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function OverviewCards() {
  const { data, isLoading } = useAnalyticsOverview();

  const cards = [
    {
      label: 'Total Tokens',
      value: data ? formatTokens(data.total_input_tokens + data.total_output_tokens) : '—',
      sub: data ? `${formatTokens(data.total_input_tokens)} in / ${formatTokens(data.total_output_tokens)} out` : '',
    },
    { label: 'Estimated Cost', value: data ? `$${data.estimated_cost_usd.toFixed(4)}` : '—', sub: 'Claude Sonnet pricing' },
    { label: 'Total Sessions', value: data ? data.total_sessions.toString() : '—', sub: `${data?.active_sessions_today ?? 0} today` },
    { label: 'Tasks with AI', value: data ? data.total_tasks_with_sessions.toString() : '—', sub: 'tasks with ≥1 session' },
  ];

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {cards.map((card) => (
        <div key={card.label} className="rounded-xl border border-border bg-card p-5 space-y-1">
          <p className="text-xs text-muted-foreground uppercase tracking-wider">{card.label}</p>
          <p className="text-2xl font-bold">
            {isLoading ? <span className="animate-pulse bg-muted rounded w-16 h-7 inline-block" /> : card.value}
          </p>
          <p className="text-xs text-muted-foreground">{card.sub}</p>
        </div>
      ))}
    </div>
  );
}
