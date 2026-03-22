'use client';

import { useEffect, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import { useComments } from '@/hooks/use-comments';
import { useTaskSessionsDetail } from '@/hooks/use-sessions';
import type { Task } from '@/types/task';
import type { CommentWithReplies } from '@/types/comment';
import { cn } from '@/lib/utils';

interface ActivityTimelineProps {
  task: Task;
  sessionId: string | null;
}

interface TimelineEntry {
  id: string;
  type: 'created' | 'stage' | 'session_started' | 'context_updated' | 'plan_created' | 'session_completed' | 'rate_limited' | 'summary';
  description: string;
  detail?: string;
  timestamp: Date;
  expandable?: boolean;
}

function DotIcon({ type }: { type: TimelineEntry['type'] }) {
  const colorMap: Record<TimelineEntry['type'], string> = {
    created: 'bg-gray-400',
    stage: 'bg-blue-500',
    session_started: 'bg-blue-500',
    context_updated: 'bg-gray-400',
    plan_created: 'bg-green-500',
    session_completed: 'bg-green-500',
    rate_limited: 'bg-amber-500',
    summary: 'bg-purple-500',
  };
  return (
    <div className={cn('h-2.5 w-2.5 rounded-full mt-1.5 shrink-0', colorMap[type])} />
  );
}

function RelativeTime({ date }: { date: Date }) {
  const [display, setDisplay] = useState('');

  useEffect(() => {
    const format = (d: Date) => {
      const diffMs = Date.now() - d.getTime();
      const diffSecs = Math.floor(diffMs / 1000);
      if (diffSecs < 60) return 'just now';
      const diffMins = Math.floor(diffSecs / 60);
      if (diffMins < 60) return `${diffMins}m ago`;
      const diffHours = Math.floor(diffMins / 60);
      if (diffHours < 24) return `${diffHours}h ago`;
      const diffDays = Math.floor(diffHours / 24);
      return `${diffDays}d ago`;
    };

    setDisplay(format(date));
    const interval = setInterval(() => setDisplay(format(date)), 30_000);
    return () => clearInterval(interval);
  }, [date]);

  return (
    <span
      className="text-xs text-muted-foreground/70 shrink-0"
      title={date.toLocaleString()}
    >
      {display}
    </span>
  );
}

function ExpandableEntry({ entry }: { entry: TimelineEntry }) {
  const [expanded, setExpanded] = useState(false);
  const preview = entry.detail ? entry.detail.slice(0, 100) : null;
  const hasMore = entry.detail && entry.detail.length > 100;

  return (
    <div className="min-w-0 flex-1">
      <div className="flex items-start justify-between gap-2">
        <span className="text-sm text-foreground">{entry.description}</span>
        <RelativeTime date={entry.timestamp} />
      </div>
      {entry.detail && (
        <div className="mt-1">
          <p className="text-xs text-muted-foreground leading-relaxed">
            {expanded ? entry.detail : preview}
            {hasMore && !expanded && '...'}
          </p>
          {hasMore && (
            <button
              onClick={() => setExpanded(!expanded)}
              className="text-xs text-blue-500 hover:underline mt-0.5"
            >
              {expanded ? 'Show less' : 'Show more'}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function buildStaticEntries(task: Task, comments: CommentWithReplies[]): TimelineEntry[] {
  const entries: TimelineEntry[] = [];

  // Always show "Task created" at the bottom
  entries.push({
    id: 'created',
    type: 'created',
    description: 'Task created',
    timestamp: new Date(task.created_at),
  });

  // Stage change hint based on current stage (show current stage milestone if not backlog)
  const stageMilestones: Record<string, string> = {
    planning: 'Moved to Planning',
    ready: 'Moved to Ready',
    in_progress: 'Moved to In Progress',
    review: 'Moved to Review',
    done: 'Completed',
  };
  if (task.stage !== 'backlog') {
    entries.push({
      id: `stage-${task.stage}`,
      type: 'stage',
      description: stageMilestones[task.stage] ?? `Stage: ${task.stage}`,
      timestamp: new Date(task.updated_at),
    });
  }

  // Comments by litellm = session summaries
  const summaries = comments
    .flatMap((c) => [c, ...c.replies])
    .filter((c) => c.author === 'litellm');

  for (const summary of summaries) {
    entries.push({
      id: `summary-${summary.id}`,
      type: 'summary',
      description: 'Session summarized',
      detail: summary.content,
      timestamp: new Date(summary.created_at),
      expandable: true,
    });
  }

  // Sort newest first
  entries.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
  return entries;
}

export function ActivityTimeline({ task, sessionId }: ActivityTimelineProps) {
  const { data: comments = [], isLoading } = useComments(task.id);
  const { subscribe } = useWebSocket();
  const [dynamicEntries, setDynamicEntries] = useState<TimelineEntry[]>([]);

  const { data: taskSessions = [] } = useTaskSessionsDetail(task.id);

  // Always subscribe to rate_limited events for this task (no sessionId guard needed)
  useEffect(() => {
    const unsub = subscribe('rate_limited', (data: unknown) => {
      const msg = data as { session_id?: string; task_id?: string; reset_at?: string };
      if (msg.task_id !== task.id) return;
      const resetTime = msg.reset_at ? new Date(msg.reset_at) : null;
      const timeStr = resetTime ? resetTime.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '';
      setDynamicEntries((prev) => {
        const id = `rate_limited_${Date.now()}`;
        if (prev.some((e) => e.id === id)) return prev;
        return [{ id, type: 'rate_limited', description: `Rate limited${timeStr ? ` — retrying at ${timeStr}` : ''}`, timestamp: new Date() }, ...prev];
      });
    });
    return unsub;
  }, [task.id, subscribe]);

  // Subscribe to session-specific WS events for live timeline updates
  useEffect(() => {
    if (!sessionId) return;

    const addEntry = (entry: TimelineEntry) => {
      setDynamicEntries((prev) => {
        // Deduplicate by id
        if (prev.some((e) => e.id === entry.id)) return prev;
        return [entry, ...prev];
      });
    };

    const unsubStageContext = subscribe('stage_context_set', (data: unknown) => {
      const msg = data as { session_id?: string; task_id?: string; mode?: string };
      if (msg.task_id !== task.id && msg.session_id !== sessionId) return;
      addEntry({
        id: `stage_context_${Date.now()}`,
        type: 'session_started',
        description: `Session started in ${msg.mode ?? 'unknown'} mode`,
        timestamp: new Date(),
      });
    });

    const unsubContextFile = subscribe('context_file_updated', (data: unknown) => {
      const msg = data as { session_id?: string; task_id?: string };
      if (msg.task_id !== task.id && msg.session_id !== sessionId) return;
      addEntry({
        id: `context_file_${Date.now()}`,
        type: 'context_updated',
        description: 'Claude.md written to project',
        timestamp: new Date(),
      });
    });

    const unsubPlan = subscribe('plan_created', (data: unknown) => {
      const msg = data as { session_id?: string; task_id?: string; preview?: string };
      if (msg.task_id !== task.id && msg.session_id !== sessionId) return;
      addEntry({
        id: `plan_${Date.now()}`,
        type: 'plan_created',
        description: 'Plan created',
        detail: msg.preview,
        timestamp: new Date(),
        expandable: !!msg.preview,
      });
    });

    const unsubStatus = subscribe('session_status', (data: unknown) => {
      const msg = data as { session_id?: string; task_id?: string; status?: string };
      if (msg.session_id !== sessionId) return;
      if (msg.status === 'completed') {
        addEntry({
          id: `completed_${Date.now()}`,
          type: 'session_completed',
          description: 'Session completed',
          timestamp: new Date(),
        });
      }
    });

    return () => {
      unsubStageContext();
      unsubContextFile();
      unsubPlan();
      unsubStatus();
    };
  }, [sessionId, task.id, subscribe]);

  const staticEntries = buildStaticEntries(task, comments);

  // Build historical rate limit entries from past sessions stored in DB
  const historicalRateLimitEntries: TimelineEntry[] = taskSessions
    .filter((s) => s.error_message?.startsWith('rate_limited:'))
    .map((s) => {
      const resetIso = s.error_message!.replace('rate_limited:', '');
      const resetTime = new Date(resetIso);
      const timeStr = resetTime.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      return {
        id: `rate_limited_${s.id}`,
        type: 'rate_limited' as const,
        description: `Rate limited — retrying at ${timeStr}`,
        timestamp: new Date(s.ended_at ?? s.started_at),
      };
    });

  // Merge and sort all entries
  const allEntries = [...dynamicEntries, ...staticEntries, ...historicalRateLimitEntries].sort(
    (a, b) => b.timestamp.getTime() - a.timestamp.getTime()
  );

  if (isLoading && allEntries.length === 0) {
    return <div className="h-16 animate-pulse bg-muted rounded" />;
  }

  return (
    <div className="space-y-0">
      {allEntries.length === 0 ? (
        <p className="text-muted-foreground italic text-sm">No activity yet</p>
      ) : (
        <div className="relative">
          {/* Vertical line */}
          <div className="absolute left-1 top-3 bottom-3 w-px bg-border" />
          <div className="space-y-4 pl-6">
            {allEntries.map((entry) => (
              <div key={entry.id} className="relative flex items-start gap-3 -ml-6">
                <DotIcon type={entry.type} />
                {entry.expandable ? (
                  <ExpandableEntry entry={entry} />
                ) : (
                  <div className="min-w-0 flex-1">
                    <div className="flex items-start justify-between gap-2">
                      <span className="text-sm text-foreground">{entry.description}</span>
                      <RelativeTime date={entry.timestamp} />
                    </div>
                    {entry.detail && (
                      <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                        {entry.detail}
                      </p>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
