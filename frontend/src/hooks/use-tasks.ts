'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import { logger } from '@/lib/logger';
import type { Task, CreateTask, UpdateTask, Stage } from '@/types/task';

export function useTasks(stage?: Stage) {
  return useQuery({
    queryKey: ['tasks', stage],
    queryFn: () =>
      apiClient<Task[]>(`/api/tasks${stage ? `?stage=${stage}` : ''}`),
  });
}

export function useTask(id: string) {
  return useQuery({
    queryKey: ['tasks', id],
    queryFn: () => apiClient<Task>(`/api/tasks/${id}`),
    enabled: !!id,
  });
}

export function useCreateTask() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateTask) =>
      apiClient<Task>('/api/tasks', {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    onSuccess: (task) => {
      logger.info('Task created', { taskId: task.id, title: task.title });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
    onError: (error) => {
      logger.error('Failed to create task', { error: String(error) });
    },
  });
}

export function useUpdateTask() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateTask }) =>
      apiClient<Task>(`/api/tasks/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(data),
      }),
    onSuccess: (task, { id }) => {
      logger.info('Task updated', { taskId: id, title: task.title });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
      queryClient.invalidateQueries({ queryKey: ['tasks', id] });
    },
    onError: (error, { id }) => {
      logger.error('Failed to update task', { taskId: id, error: String(error) });
    },
  });
}

export function useDeleteTask() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) =>
      apiClient<void>(`/api/tasks/${id}`, {
        method: 'DELETE',
      }),
    onSuccess: (_, id) => {
      logger.info('Task deleted', { taskId: id });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
    onError: (error, id) => {
      logger.error('Failed to delete task', { taskId: id, error: String(error) });
    },
  });
}
