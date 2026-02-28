'use client';

import { Badge } from '@/components/ui/badge';

type SessionStatus = 'pending' | 'running' | 'completed' | 'failed';

const statusColors: Record<SessionStatus, string> = {
  pending: 'bg-yellow-500',
  running: 'bg-blue-500 animate-pulse',
  completed: 'bg-green-500',
  failed: 'bg-red-500',
};

const statusLabels: Record<SessionStatus, string> = {
  pending: 'Pending',
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
};

interface SessionStatusBadgeProps {
  status: SessionStatus;
  sessionId?: string;
}

export function SessionStatusBadge({ status, sessionId }: SessionStatusBadgeProps) {
  return (
    <div className="flex items-center gap-2">
      <Badge className={`${statusColors[status]} text-white`}>
        {statusLabels[status]}
      </Badge>
      {sessionId && (
        <span className="text-xs text-muted-foreground font-mono">
          {sessionId.slice(0, 8)}
        </span>
      )}
    </div>
  );
}
