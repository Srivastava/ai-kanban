'use client';

import { Play, Square, RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { apiClient } from '@/lib/api-client';
import { useQueryClient } from '@tanstack/react-query';
import { logger } from '@/lib/logger';
import type { SessionStatus } from '@/types/session';

interface SessionControlsProps {
  taskId: string;
  sessionId?: string | null;
  status?: SessionStatus | null;
  hasClaudeComments?: boolean;
}

export function SessionControls({
  taskId,
  sessionId,
  status,
  hasClaudeComments = false,
}: SessionControlsProps) {
  const queryClient = useQueryClient();

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
    queryClient.invalidateQueries({ queryKey: ['sessions'] });
    queryClient.invalidateQueries({ queryKey: ['comments', taskId] });
  };

  const log = logger.withContext({ task_id: taskId, session_id: sessionId ?? undefined });

  const startSession = async () => {
    log.info('Starting new Claude session', { task_id: taskId });
    try {
      await apiClient(`/api/tasks/${taskId}/sessions`, { method: 'POST' });
      log.info('Session started successfully', { task_id: taskId });
      invalidate();
    } catch (err) {
      log.error('Failed to start session', { task_id: taskId, error: String(err) });
    }
  };

  const continueSession = async () => {
    log.info('Continuing Claude session', { task_id: taskId, session_id: sessionId });
    try {
      await apiClient(`/api/tasks/${taskId}/sessions/continue`, { method: 'POST' });
      log.info('Continue session enqueued', { task_id: taskId, session_id: sessionId });
      invalidate();
    } catch (err) {
      log.error('Failed to continue session', { task_id: taskId, session_id: sessionId, error: String(err) });
    }
  };

  const stopSession = async () => {
    if (!sessionId) return;
    log.info('Stopping session', { task_id: taskId, session_id: sessionId });
    try {
      await apiClient(`/api/sessions/${sessionId}/stop`, { method: 'POST' });
      log.info('Session stopped', { task_id: taskId, session_id: sessionId });
      invalidate();
    } catch (err) {
      log.error('Failed to stop session', { task_id: taskId, session_id: sessionId, error: String(err) });
    }
  };

  if (status === 'pending') {
    return (
      <Button disabled size="sm">
        <Play className="mr-2 h-4 w-4" />
        Starting...
      </Button>
    );
  }

  if (status === 'running') {
    return (
      <Button onClick={stopSession} variant="destructive" size="sm">
        <Square className="mr-2 h-4 w-4" />
        Stop Session
      </Button>
    );
  }

  // completed, failed, stopped, or no session
  return (
    <div className="flex gap-2">
      <Button onClick={startSession} size="sm">
        <Play className="mr-2 h-4 w-4" />
        Start Session
      </Button>
      {hasClaudeComments && (
        <Button onClick={continueSession} variant="outline" size="sm">
          <RotateCcw className="mr-2 h-4 w-4" />
          Continue Session
        </Button>
      )}
    </div>
  );
}
