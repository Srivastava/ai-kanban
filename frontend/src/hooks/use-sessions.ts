'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient, ApiError } from '@/lib/api-client';
import type { Session } from '@/types/session';
import type { SessionDetail } from '@/types/analytics';

export function useSession(sessionId: string | null | undefined) {
  return useQuery({
    queryKey: ['sessions', sessionId],
    queryFn: async () => {
      try {
        return await apiClient<Session>(`/api/sessions/${sessionId}`);
      } catch (e) {
        if (e instanceof ApiError && e.status === 404) return null;
        throw e;
      }
    },
    enabled: !!sessionId,
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      if (status === 'running' || status === 'pending') return 3000;
      return false;
    },
  });
}

export function useAllSessions(statuses?: string[], limit = 100) {
  const qs = [
    statuses?.length ? `status=${statuses.join(',')}` : '',
    `limit=${limit}`,
  ].filter(Boolean).join('&');
  return useQuery({
    queryKey: ['sessions', 'all', statuses, limit],
    queryFn: () => apiClient<Session[]>(`/api/sessions/all?${qs}`),
    refetchInterval: 10_000,
  });
}

export function useTaskSessionsDetail(taskId: string) {
  return useQuery({
    queryKey: ['task-sessions-detail', taskId],
    queryFn: () => apiClient<SessionDetail[]>(`/api/tasks/${taskId}/sessions-detail`),
  });
}

export function useStopSession(taskId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (sessionId: string) =>
      apiClient<void>(`/api/sessions/${sessionId}/stop`, { method: 'POST' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
      queryClient.invalidateQueries({ queryKey: ['sessions'] });
    },
  });
}
