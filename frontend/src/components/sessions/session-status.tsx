'use client';

import { Badge } from '@/components/ui/badge';

type SessionStatus = 'pending' | 'running' | 'completed' | 'failed';

const statusColors: Record<SessionStatus, string> = {
  pending:   'bg-amber-100 text-amber-800 border border-amber-300 dark:bg-amber-900/40 dark:text-amber-300 dark:border-amber-700',
  running:   'bg-blue-500 text-white motion-safe:animate-breathe',
  completed: 'bg-emerald-100 text-emerald-800 border border-emerald-300 dark:bg-emerald-900/40 dark:text-emerald-300 dark:border-emerald-700',
  failed:    'bg-red-500 text-white',
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
      <Badge className={statusColors[status]}>
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
