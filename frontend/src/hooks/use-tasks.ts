'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import { logger } from '@/lib/logger';
import type { Task, CreateTask, UpdateTask, Stage } from '@/types/task';

export function useTasks(stage?: Stage) {
  logger.debug('useTasks hook called', { stage });

  return useQuery({
    queryKey: ['tasks', stage],
    queryFn: async () => {
      logger.debug('useTasks: fetching tasks', { stage });
      const endpoint = `/api/tasks${stage ? `?stage=${stage}` : ''}`;
      logger.debug('useTasks: API endpoint', { endpoint });
      const result = await apiClient<Task[]>(endpoint);
      logger.debug('useTasks: fetch complete', { count: result.length, stage });
      return result;
    },
  });
}

export function useTask(id: string) {
  logger.debug('useTask hook called', { id, enabled: !!id });

  return useQuery({
    queryKey: ['tasks', id],
    queryFn: async () => {
      logger.debug('useTask: fetching single task', { id });
      const result = await apiClient<Task>(`/api/tasks/${id}`);
      logger.debug('useTask: fetch complete', { id, title: result.title });
      return result;
    },
    enabled: !!id,
  });
}

export function useCreateTask() {
  const queryClient = useQueryClient();
  logger.debug('useCreateTask hook initialized');

  return useMutation({
    mutationFn: async (data: CreateTask) => {
      logger.debug('useCreateTask: creating task', { title: data.title, project_path: data.project_path });
      const result = await apiClient<Task>('/api/tasks', {
        method: 'POST',
        body: JSON.stringify(data),
      });
      logger.debug('useCreateTask: task created', { id: result.id, title: result.title });
      return result;
    },
    onSuccess: (task) => {
      logger.info('Task created successfully', { taskId: task.id, title: task.title });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
    onError: (error) => {
      logger.error('Failed to create task', { error: String(error), errorType: error?.constructor?.name });
    },
  });
}

export function useUpdateTask() {
  const queryClient = useQueryClient();
  logger.debug('useUpdateTask hook initialized');

  return useMutation({
    mutationFn: async ({ id, data }: { id: string; data: UpdateTask }) => {
      logger.debug('useUpdateTask: updating task', { id, updates: Object.keys(data) });
      const result = await apiClient<Task>(`/api/tasks/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(data),
      });
      logger.debug('useUpdateTask: task updated', { id, title: result.title });
      return result;
    },
    onSuccess: (task, { id }) => {
      logger.info('Task updated successfully', { taskId: id, title: task.title });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
      queryClient.invalidateQueries({ queryKey: ['tasks', id] });
    },
    onError: (error, { id }) => {
      logger.error('Failed to update task', { taskId: id, error: String(error), errorType: error?.constructor?.name });
    },
  });
}

export function useDeleteTask() {
  const queryClient = useQueryClient();
  logger.debug('useDeleteTask hook initialized');

  return useMutation({
    mutationFn: async (id: string) => {
      logger.debug('useDeleteTask: deleting task', { id });
      const result = await apiClient<void>(`/api/tasks/${id}`, {
        method: 'DELETE',
      });
      logger.debug('useDeleteTask: task deleted', { id });
      return result;
    },
    onSuccess: (_, id) => {
      logger.info('Task deleted successfully', { taskId: id });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
    onError: (error, id) => {
      logger.error('Failed to delete task', { taskId: id, error: String(error), errorType: error?.constructor?.name });
    },
  });
}
