'use client';

import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/contexts/websocket-context';

export function useTaskSubscriptions() {
  const queryClient = useQueryClient();
  const { subscribe, status } = useWebSocket();

  useEffect(() => {
    if (status !== 'connected') return;

    // task_updated is handled globally in websocket-context (setQueryData + invalidate)
    const unsubTaskCreated = subscribe('task_created', () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    });

    const unsubTaskDeleted = subscribe('task_deleted', () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    });

    return () => {
      unsubTaskCreated();
      unsubTaskDeleted();
    };
  }, [status, subscribe, queryClient]);
}
