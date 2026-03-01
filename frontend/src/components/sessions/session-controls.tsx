'use client';

import { Play, Square, RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { apiClient } from '@/lib/api-client';
import { useQueryClient } from '@tanstack/react-query';
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

  const startSession = async () => {
    await apiClient(`/api/tasks/${taskId}/sessions`, { method: 'POST' });
    invalidate();
  };

  const continueSession = async () => {
    await apiClient(`/api/tasks/${taskId}/sessions/continue`, { method: 'POST' });
    invalidate();
  };

  const stopSession = async () => {
    if (!sessionId) return;
    await apiClient(`/api/sessions/${sessionId}/stop`, { method: 'POST' });
    invalidate();
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
