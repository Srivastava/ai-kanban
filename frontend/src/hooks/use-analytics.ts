'use client';

import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type {
  AnalyticsOverview, DailyTokens, EfficiencyRow, LanguageTokens,
  MonthlyTokens, SessionTimelineEvent, SessionTokens, TaskTokens,
  ToolTokens, WeeklyTokens,
} from '@/types/analytics';

export function useAnalyticsOverview() {
  return useQuery({
    queryKey: ['analytics', 'overview'],
    queryFn: () => apiClient<AnalyticsOverview>('/api/analytics/overview'),
    refetchInterval: 30_000,
  });
}

export function useDailyTokens(days = 30) {
  return useQuery({
    queryKey: ['analytics', 'daily', days],
    queryFn: () => apiClient<DailyTokens[]>(`/api/analytics/tokens/daily?days=${days}`),
  });
}

export function useWeeklyTokens(weeks = 12) {
  return useQuery({
    queryKey: ['analytics', 'weekly', weeks],
    queryFn: () => apiClient<WeeklyTokens[]>(`/api/analytics/tokens/weekly?weeks=${weeks}`),
  });
}

export function useMonthlyTokens(months = 6) {
  return useQuery({
    queryKey: ['analytics', 'monthly', months],
    queryFn: () => apiClient<MonthlyTokens[]>(`/api/analytics/tokens/monthly?months=${months}`),
  });
}

export function useTokensByTask() {
  return useQuery({
    queryKey: ['analytics', 'by-task'],
    queryFn: () => apiClient<TaskTokens[]>('/api/analytics/tokens/by-task'),
  });
}

export function useTokensBySession() {
  return useQuery({
    queryKey: ['analytics', 'by-session'],
    queryFn: () => apiClient<SessionTokens[]>('/api/analytics/tokens/by-session'),
  });
}

export function useTokensByTool() {
  return useQuery({
    queryKey: ['analytics', 'by-tool'],
    queryFn: () => apiClient<ToolTokens[]>('/api/analytics/tokens/by-tool'),
  });
}

export function useTokensByLanguage() {
  return useQuery({
    queryKey: ['analytics', 'by-language'],
    queryFn: () => apiClient<LanguageTokens[]>('/api/analytics/tokens/by-language'),
  });
}

export function useTokenEfficiency() {
  return useQuery({
    queryKey: ['analytics', 'efficiency'],
    queryFn: () => apiClient<EfficiencyRow[]>('/api/analytics/tokens/efficiency'),
  });
}

export function useSessionTimeline(sessionId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'timeline', sessionId],
    queryFn: () => apiClient<SessionTimelineEvent[]>(`/api/analytics/sessions/${sessionId}/timeline`),
    enabled: !!sessionId,
  });
}
