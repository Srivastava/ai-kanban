'use client';

import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import { logger } from '@/lib/logger';
import type {
  AnalyticsOverview, DailyTokens, EfficiencyRow, LanguageTokens,
  MonthlyTokens, SessionTimelineEvent, SessionTokens, TaskTokens,
  ToolTokens, WeeklyTokens,
} from '@/types/analytics';

export function useAnalyticsOverview() {
  logger.debug('useAnalyticsOverview hook called');

  return useQuery({
    queryKey: ['analytics', 'overview'],
    queryFn: async () => {
      logger.debug('useAnalyticsOverview: fetching overview');
      const result = await apiClient<AnalyticsOverview>('/api/analytics/overview');
      logger.debug('useAnalyticsOverview: fetch complete', {
        totalSessions: result.total_sessions,
        totalInputTokens: result.total_input_tokens,
        totalOutputTokens: result.total_output_tokens,
      });
      return result;
    },
    refetchInterval: 30_000,
  });
}

export function useDailyTokens(days = 30) {
  logger.debug('useDailyTokens hook called', { days });

  return useQuery({
    queryKey: ['analytics', 'daily', days],
    queryFn: async () => {
      logger.debug('useDailyTokens: fetching daily tokens', { days });
      const result = await apiClient<DailyTokens[]>(`/api/analytics/tokens/daily?days=${days}`);
      logger.debug('useDailyTokens: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useWeeklyTokens(weeks = 12) {
  logger.debug('useWeeklyTokens hook called', { weeks });

  return useQuery({
    queryKey: ['analytics', 'weekly', weeks],
    queryFn: async () => {
      logger.debug('useWeeklyTokens: fetching weekly tokens', { weeks });
      const result = await apiClient<WeeklyTokens[]>(`/api/analytics/tokens/weekly?weeks=${weeks}`);
      logger.debug('useWeeklyTokens: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useMonthlyTokens(months = 6) {
  logger.debug('useMonthlyTokens hook called', { months });

  return useQuery({
    queryKey: ['analytics', 'monthly', months],
    queryFn: async () => {
      logger.debug('useMonthlyTokens: fetching monthly tokens', { months });
      const result = await apiClient<MonthlyTokens[]>(`/api/analytics/tokens/monthly?months=${months}`);
      logger.debug('useMonthlyTokens: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useTokensByTask() {
  logger.debug('useTokensByTask hook called');

  return useQuery({
    queryKey: ['analytics', 'by-task'],
    queryFn: async () => {
      logger.debug('useTokensByTask: fetching tokens by task');
      const result = await apiClient<TaskTokens[]>('/api/analytics/tokens/by-task');
      logger.debug('useTokensByTask: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useTokensBySession() {
  logger.debug('useTokensBySession hook called');

  return useQuery({
    queryKey: ['analytics', 'by-session'],
    queryFn: async () => {
      logger.debug('useTokensBySession: fetching tokens by session');
      const result = await apiClient<SessionTokens[]>('/api/analytics/tokens/by-session');
      logger.debug('useTokensBySession: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useTokensByTool() {
  logger.debug('useTokensByTool hook called');

  return useQuery({
    queryKey: ['analytics', 'by-tool'],
    queryFn: async () => {
      logger.debug('useTokensByTool: fetching tokens by tool');
      const result = await apiClient<ToolTokens[]>('/api/analytics/tokens/by-tool');
      logger.debug('useTokensByTool: fetch complete', { count: result.length, tools: result.map(t => t.tool_name) });
      return result;
    },
  });
}

export function useTokensByLanguage() {
  logger.debug('useTokensByLanguage hook called');

  return useQuery({
    queryKey: ['analytics', 'by-language'],
    queryFn: async () => {
      logger.debug('useTokensByLanguage: fetching tokens by language');
      const result = await apiClient<LanguageTokens[]>('/api/analytics/tokens/by-language');
      logger.debug('useTokensByLanguage: fetch complete', { count: result.length, languages: result.map(l => l.file_ext) });
      return result;
    },
  });
}

export function useTokenEfficiency() {
  logger.debug('useTokenEfficiency hook called');

  return useQuery({
    queryKey: ['analytics', 'efficiency'],
    queryFn: async () => {
      logger.debug('useTokenEfficiency: fetching token efficiency');
      const result = await apiClient<EfficiencyRow[]>('/api/analytics/tokens/efficiency');
      logger.debug('useTokenEfficiency: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useSessionTimeline(sessionId: string | null) {
  logger.debug('useSessionTimeline hook called', { sessionId, enabled: !!sessionId });

  return useQuery({
    queryKey: ['analytics', 'timeline', sessionId],
    queryFn: async () => {
      logger.debug('useSessionTimeline: fetching timeline', { sessionId });
      const result = await apiClient<SessionTimelineEvent[]>(`/api/analytics/sessions/${sessionId}/timeline`);
      logger.debug('useSessionTimeline: fetch complete', { sessionId, count: result.length });
      return result;
    },
    enabled: !!sessionId,
  });
}
