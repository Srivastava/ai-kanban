'use client';

import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import { logger } from '@/lib/logger';
import type {
  AnalyticsOverview, BurnRate, CostByTask, DailyTokens, DevActivityRow, EfficiencyRow, LanguageTokens,
  LocHistoryEntry, MonthlyTokens, SessionDetail, SessionSummary, SessionTimelineEvent, TaskTimelineEvent, SessionTokens, TaskTokens,
  TokensByStage, ToolTokens, WeeklyTokens, UsageWindows, PlanTier, RoiMetrics, ContextWindowUsage,
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

export function useDailyTokens(days = 30, taskId?: string | null) {
  logger.debug('useDailyTokens hook called', { days, taskId });

  return useQuery({
    queryKey: ['analytics', 'daily', days, taskId],
    queryFn: async () => {
      logger.debug('useDailyTokens: fetching daily tokens', { days, taskId });
      const params = taskId ? `days=${days}&task_id=${taskId}` : `days=${days}`;
      const result = await apiClient<DailyTokens[]>(`/api/analytics/tokens/daily?${params}`);
      logger.debug('useDailyTokens: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useWeeklyTokens(weeks = 12, taskId?: string | null) {
  logger.debug('useWeeklyTokens hook called', { weeks, taskId });

  return useQuery({
    queryKey: ['analytics', 'weekly', weeks, taskId],
    queryFn: async () => {
      logger.debug('useWeeklyTokens: fetching weekly tokens', { weeks, taskId });
      const params = taskId ? `weeks=${weeks}&task_id=${taskId}` : `weeks=${weeks}`;
      const result = await apiClient<WeeklyTokens[]>(`/api/analytics/tokens/weekly?${params}`);
      logger.debug('useWeeklyTokens: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useMonthlyTokens(months = 6, taskId?: string | null) {
  logger.debug('useMonthlyTokens hook called', { months, taskId });

  return useQuery({
    queryKey: ['analytics', 'monthly', months, taskId],
    queryFn: async () => {
      logger.debug('useMonthlyTokens: fetching monthly tokens', { months, taskId });
      const params = taskId ? `months=${months}&task_id=${taskId}` : `months=${months}`;
      const result = await apiClient<MonthlyTokens[]>(`/api/analytics/tokens/monthly?${params}`);
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

export function useTokensByTool(taskId?: string | null) {
  logger.debug('useTokensByTool hook called', { taskId });

  return useQuery({
    queryKey: ['analytics', 'by-tool', taskId],
    queryFn: async () => {
      logger.debug('useTokensByTool: fetching tokens by tool');
      const url = taskId ? `/api/analytics/tokens/by-tool?task_id=${taskId}` : '/api/analytics/tokens/by-tool';
      const result = await apiClient<ToolTokens[]>(url);
      logger.debug('useTokensByTool: fetch complete', { count: result.length, tools: result.map(t => t.tool_name) });
      return result;
    },
  });
}

export function useTokensByLanguage(taskId?: string | null) {
  logger.debug('useTokensByLanguage hook called', { taskId });

  return useQuery({
    queryKey: ['analytics', 'by-language', taskId],
    queryFn: async () => {
      logger.debug('useTokensByLanguage: fetching tokens by language');
      const url = taskId ? `/api/analytics/tokens/by-language?task_id=${taskId}` : '/api/analytics/tokens/by-language';
      const result = await apiClient<LanguageTokens[]>(url);
      logger.debug('useTokensByLanguage: fetch complete', { count: result.length, languages: result.map(l => l.file_ext) });
      return result;
    },
  });
}

export function useTokenEfficiency(taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'efficiency', taskId],
    queryFn: async () => {
      const url = taskId
        ? `/api/analytics/tokens/efficiency?task_id=${taskId}`
        : '/api/analytics/tokens/efficiency';
      const result = await apiClient<EfficiencyRow[]>(url);
      logger.debug('useTokenEfficiency: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useLocHistory(taskId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'loc-history', taskId],
    queryFn: () => apiClient<LocHistoryEntry[]>(`/api/analytics/tasks/${taskId!}/loc-history`),
    enabled: !!taskId,
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
export function useUsageWindows() {
  logger.debug('useUsageWindows hook called');

  return useQuery({
    queryKey: ['analytics', 'usage-windows'],
    queryFn: async () => {
      const result = await apiClient<UsageWindows>('/api/analytics/usage-windows');
      logger.debug('useUsageWindows: fetch complete', { tokens_5hr: result.tokens_5hr });
      return result;
    },
    refetchInterval: 60_000, // refresh every minute
  });
}

export function useCostByTask() {
  return useQuery({
    queryKey: ['analytics', 'cost-by-task'],
    queryFn: async () => {
      const result = await apiClient<CostByTask[]>('/api/analytics/cost/by-task');
      logger.debug('useCostByTask: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useTokensByStage(taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'by-stage', taskId],
    queryFn: async () => {
      const url = taskId ? `/api/analytics/tokens/by-stage?task_id=${taskId}` : '/api/analytics/tokens/by-stage';
      const result = await apiClient<TokensByStage[]>(url);
      logger.debug('useTokensByStage: fetch complete', { count: result.length });
      return result;
    },
  });
}

export function useSessionSummary() {
  return useQuery({
    queryKey: ['analytics', 'session-summary'],
    queryFn: async () => {
      const result = await apiClient<SessionSummary>('/api/analytics/sessions/summary');
      logger.debug('useSessionSummary: fetch complete', { total: result.total_sessions });
      return result;
    },
    refetchInterval: 30_000,
  });
}

export function useBurnRate() {
  return useQuery({
    queryKey: ['analytics', 'burn-rate'],
    queryFn: async () => {
      const result = await apiClient<BurnRate>('/api/analytics/burn-rate');
      logger.debug('useBurnRate: fetch complete', { tph: result.tokens_last_hour });
      return result;
    },
    refetchInterval: 60_000,
  });
}

export function useTaskTimeline(taskId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'task-timeline', taskId],
    queryFn: () => apiClient<TaskTimelineEvent[]>(`/api/analytics/tasks/${taskId}/task-timeline`),
    enabled: !!taskId,
  });
}

export function useTaskSessions(taskId: string | null) {
  return useQuery({
    queryKey: ['sessions', 'by-task', taskId],
    queryFn: () => apiClient<SessionDetail[]>(`/api/tasks/${taskId}/sessions-detail`),
    enabled: !!taskId,
    refetchInterval: 15_000,
  });
}

export function useDevActivity(taskId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'dev-activity', taskId],
    queryFn: () => apiClient<DevActivityRow[]>(
      taskId ? `/api/analytics/dev-activity?task_id=${taskId}` : '/api/analytics/dev-activity'
    ),
    enabled: !!taskId,
    refetchInterval: 30_000,
  });
}

export function usePlanTier() {
  return useQuery({
    queryKey: ['analytics', 'plan-tier'],
    queryFn: () => apiClient<PlanTier>('/api/analytics/plan-tier'),
    // No refetchInterval — static/env-driven
  });
}

export function useRoiMetrics(taskId?: string | null) {
  return useQuery({
    queryKey: ['analytics', 'roi', taskId],
    queryFn: () => apiClient<RoiMetrics>(
      taskId ? `/api/analytics/roi?task_id=${taskId}` : '/api/analytics/roi'
    ),
    refetchInterval: 60_000,
  });
}

export function useContextUsage() {
  return useQuery({
    queryKey: ['analytics', 'context-usage'],
    queryFn: () => apiClient<ContextWindowUsage[]>('/api/analytics/context-usage'),
    refetchInterval: 15_000,
  });
}
