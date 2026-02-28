'use client';

import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/contexts/websocket-context';

export function useTaskSubscriptions() {
  const queryClient = useQueryClient();
  const { subscribe, status } = useWebSocket();

  useEffect(() => {
    if (status !== 'connected') return;

    const unsubTaskUpdated = subscribe('task_updated', (data: unknown) => {
      const task = (data as { task: { id: string } }).task;
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
      queryClient.invalidateQueries({ queryKey: ['tasks', task.id] });
    });

    const unsubTaskCreated = subscribe('task_created', () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    });

    const unsubTaskDeleted = subscribe('task_deleted', () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    });

    return () => {
      unsubTaskUpdated();
      unsubTaskCreated();
      unsubTaskDeleted();
    };
  }, [status, subscribe, queryClient]);
}
