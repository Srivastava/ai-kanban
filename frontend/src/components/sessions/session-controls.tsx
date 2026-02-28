'use client';

import { Play, Pause, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { apiClient } from '@/lib/api-client';
import { useQueryClient } from '@tanstack/react-query';

type SessionStatus = 'pending' | 'running' | 'completed' | 'failed' | 'paused';

interface SessionControlsProps {
  taskId: string;
  sessionId?: string;
  status?: SessionStatus;
}

export function SessionControls({ taskId, sessionId, status }: SessionControlsProps) {
  const queryClient = useQueryClient();

  const startSession = async () => {
    await apiClient(`/api/tasks/${taskId}/sessions`, { method: 'POST' });
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
  };

  const pauseSession = async () => {
    if (!sessionId) return;
    await apiClient(`/api/sessions/${sessionId}/pause`, { method: 'POST' });
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
  };

  const resumeSession = async () => {
    if (!sessionId) return;
    await apiClient(`/api/sessions/${sessionId}/resume`, { method: 'POST' });
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
  };

  const stopSession = async () => {
    if (!sessionId) return;
    await apiClient(`/api/sessions/${sessionId}/stop`, { method: 'POST' });
    queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
  };

  if (!status || status === 'completed' || status === 'failed') {
    return (
      <Button onClick={startSession} size="sm">
        <Play className="mr-2 h-4 w-4" />
        Start Claude Session
      </Button>
    );
  }

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
      <div className="flex gap-2">
        <Button onClick={pauseSession} variant="outline" size="sm">
          <Pause className="mr-2 h-4 w-4" />
          Pause
        </Button>
        <Button onClick={stopSession} variant="destructive" size="sm">
          <Square className="mr-2 h-4 w-4" />
          Stop
        </Button>
      </div>
    );
  }

  // paused
  return (
    <div className="flex gap-2">
      <Button onClick={resumeSession} size="sm">
        <Play className="mr-2 h-4 w-4" />
        Resume
      </Button>
      <Button onClick={stopSession} variant="destructive" size="sm">
        <Square className="mr-2 h-4 w-4" />
        Stop
      </Button>
    </div>
  );
}
