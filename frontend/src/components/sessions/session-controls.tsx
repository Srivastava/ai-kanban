'use client';

import { useState } from 'react';
import { Play, Square, RotateCcw, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { apiClient, ApiError } from '@/lib/api-client';
import { useQueryClient } from '@tanstack/react-query';
import { logger } from '@/lib/logger';
import type { SessionStatus } from '@/types/session';

interface SessionControlsProps {
  taskId: string;
  sessionId?: string | null;
  status?: SessionStatus | null;
  hasClaudeComments?: boolean;
}

function parseApiError(err: unknown): string {
  if (err instanceof ApiError) {
    try {
      const parsed = JSON.parse(err.message);
      if (parsed?.error) return parsed.error;
    } catch {}
    return err.message;
  }
  return String(err);
}

export function SessionControls({ taskId, sessionId, status, hasClaudeComments = false }: SessionControlsProps) {
  const queryClient = useQueryClient();
  const [sessionError, setSessionError] = useState<string | null>(null);

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
    queryClient.invalidateQueries({ queryKey: ['sessions'] });
    queryClient.invalidateQueries({ queryKey: ['comments', taskId] });
  };

  const log = logger.withContext({ task_id: taskId, session_id: sessionId ?? undefined });

  const startSession = async () => {
    setSessionError(null);
    log.info('Starting new Claude session', { task_id: taskId });
    try {
      await apiClient(`/api/tasks/${taskId}/sessions`, { method: 'POST' });
      log.info('Session started successfully', { task_id: taskId });
      invalidate();
    } catch (err) {
      log.error('Failed to start session', { task_id: taskId, error: String(err) });
      setSessionError(parseApiError(err));
    }
  };

  const continueSession = async () => {
    setSessionError(null);
    log.info('Continuing Claude session', { task_id: taskId, session_id: sessionId });
    try {
      await apiClient(`/api/tasks/${taskId}/sessions/continue`, { method: 'POST' });
      log.info('Continue session enqueued', { task_id: taskId, session_id: sessionId });
      invalidate();
    } catch (err) {
      log.error('Failed to continue session', { task_id: taskId, session_id: sessionId, error: String(err) });
      setSessionError(parseApiError(err));
    }
  };

  const stopSession = async () => {
    if (!sessionId) return;
    setSessionError(null);
    log.info('Stopping session', { task_id: taskId, session_id: sessionId });
    try {
      await apiClient(`/api/sessions/${sessionId}/stop`, { method: 'POST' });
      log.info('Session stopped', { task_id: taskId, session_id: sessionId });
      invalidate();
    } catch (err) {
      log.error('Failed to stop session', { task_id: taskId, session_id: sessionId, error: String(err) });
      setSessionError(parseApiError(err));
    }
  };

  return (
    <div className="space-y-3">
      {status === 'pending' && (
        <Button disabled size="sm">
          <Play className="mr-2 h-4 w-4" />
          Starting...
        </Button>
      )}
      {status === 'running' && (
        <Button onClick={stopSession} variant="destructive" size="sm">
          <Square className="mr-2 h-4 w-4" />
          Stop Session
        </Button>
      )}
      {(status === 'completed' || status === 'failed' || status === 'stopped' || !status) && (
        <div className="flex flex-wrap gap-2">
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
      )}
      {sessionError && (
        <div className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
          <span>{sessionError}</span>
        </div>
      )}
    </div>
  );
}
