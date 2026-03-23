'use client';
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type { AnalyticsOverview } from '@/types/analytics';

function fmt(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(Math.round(n));
}

export function useSidebarMetrics() {
  const { data } = useQuery<AnalyticsOverview>({
    queryKey: ['analytics', 'overview'],
    queryFn: () => apiClient<AnalyticsOverview>('/api/analytics/overview'),
    staleTime: 60_000,
    refetchInterval: 60_000,
  });

  if (!data) return null;

  const totalTokens =
    data.total_input_tokens +
    data.total_output_tokens +
    data.total_cache_creation_tokens +
    data.total_cache_read_tokens;

  const avgCostPerSession = data.total_sessions > 0
    ? `$${(data.estimated_cost_usd / data.total_sessions).toFixed(2)}`
    : '—';

  // Cache hit: cache_read tokens as percentage of total tokens
  const cacheReadSavings = totalTokens > 0
    ? Math.round((data.total_cache_read_tokens / totalTokens) * 100)
    : 0;

  return [
    { label: 'Total Cost', value: `$${data.estimated_cost_usd.toFixed(2)}` },
    { label: 'Sessions', value: String(data.total_sessions) },
    { label: 'Tokens', value: fmt(totalTokens) },
    { label: 'Tasks w/ AI', value: String(data.total_tasks_with_sessions) },
    { label: 'Avg / Session', value: avgCostPerSession },
    { label: 'Cache Hit', value: `${cacheReadSavings}%` },
  ];
}
