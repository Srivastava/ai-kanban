'use client';

import { Suspense, useState } from 'react';
import { useSearchParams } from 'next/navigation';
import { useQuery } from '@tanstack/react-query';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { TaskList } from '@/components/tasks/task-list';
import { CreateTaskDialog } from '@/components/tasks/create-task-dialog';
import { useTasks } from '@/hooks/use-tasks';
import { useAllSessions } from '@/hooks/use-sessions';
import { apiClient } from '@/lib/api-client';
import type { Stage } from '@/types/task';
import type { CostByTask } from '@/types/analytics';

// ── stats strip ───────────────────────────────────────────────────────────────

function StatsStrip({ onNewTask }: { onNewTask: () => void }) {
  const { data: activeSessions = [] } = useAllSessions(['running', 'pending']);
  const { data: allTasks = [] } = useTasks();
  const { data: costData = [] } = useQuery<CostByTask[]>({
    queryKey: ['cost-by-task'],
    queryFn: () => apiClient<CostByTask[]>('/api/analytics/cost/by-task'),
    staleTime: 60_000,
  });

  // Rate-limited sessions (stopped with rate_limited: prefix, reset time still in the future)
  const { data: stoppedSessions = [] } = useAllSessions(['stopped']);
  const rateLimitedSession = stoppedSessions.find((s) => {
    if (!s.error_message?.startsWith('rate_limited:')) return false;
    const resetAt = s.error_message.replace('rate_limited:', '').trim();
    return resetAt && new Date(resetAt) > new Date();
  });

  const activeCount = activeSessions.length;
  const inReviewCount = allTasks.filter((t) => t.stage === 'review').length;

  // Today's cost: sum cost_usd from costData — API doesn't filter by date so sum all
  // (the API returns cost by task total; we approximate "today" by summing all available)
  const todayCost = costData.reduce((sum, c) => sum + (c.cost_usd ?? 0), 0);

  const hasStats = activeCount > 0 || inReviewCount > 0 || todayCost > 0;

  return (
    <>
      {/* Rate limit banner */}
      {rateLimitedSession && (
        <div className="mb-3 flex items-center gap-2 rounded-md bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800 px-4 py-2 text-sm text-amber-800 dark:text-amber-300">
          <span>⏸</span>
          <span>
            Rate limited
            {rateLimitedSession.error_message?.replace('rate_limited:', '') && (
              <> — retrying at {rateLimitedSession.error_message.replace('rate_limited:', '').trim()}</>
            )}
          </span>
        </div>
      )}

      {/* Stats strip */}
      {hasStats && (
        <div className="mb-4 flex flex-wrap items-center gap-2">
          {activeCount > 0 && (
            <span className="text-xs bg-muted/60 rounded-md px-3 py-1.5 flex items-center gap-1.5 text-foreground">
              ⚡ <strong>{activeCount}</strong> active
            </span>
          )}
          {inReviewCount > 0 && (
            <span className="text-xs bg-muted/60 rounded-md px-3 py-1.5 flex items-center gap-1.5 text-foreground">
              👁 <strong>{inReviewCount}</strong> in review
            </span>
          )}
          {todayCost > 0 && (
            <span className="text-xs bg-muted/60 rounded-md px-3 py-1.5 flex items-center gap-1.5 text-foreground">
              💰 <strong>${todayCost.toFixed(2)}</strong> total cost
            </span>
          )}
        </div>
      )}
    </>
  );
}

// ── task content ──────────────────────────────────────────────────────────────

function TaskContent({ onNewTask }: { onNewTask: () => void }) {
  const searchParams = useSearchParams();
  const stageParam = searchParams.get('stage');
  const stage = stageParam && stageParam !== 'all' ? (stageParam as Stage) : undefined;

  const { data: tasks = [], isLoading, error } = useTasks(stage);

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-destructive">
          Error loading tasks: {error.message}
        </p>
      </div>
    );
  }

  return (
    <TaskList
      tasks={tasks}
      isLoading={isLoading}
      stageParam={stageParam ?? undefined}
      onNewTask={onNewTask}
    />
  );
}

// ── home ──────────────────────────────────────────────────────────────────────

function HomeContent() {
  const [dialogOpen, setDialogOpen] = useState(false);

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <Header onNewTask={() => setDialogOpen(true)} />
        <main className="flex-1 p-4 sm:p-6 pb-20 md:pb-6">
          <StatsStrip onNewTask={() => setDialogOpen(true)} />
          <Suspense fallback={<div className="flex items-center justify-center h-64">Loading...</div>}>
            <TaskContent onNewTask={() => setDialogOpen(true)} />
          </Suspense>
        </main>
      </div>
      <CreateTaskDialog open={dialogOpen} onOpenChange={setDialogOpen} />
    </div>
  );
}

export default function Home() {
  return (
    <Suspense fallback={<div className="flex min-h-screen items-center justify-center">Loading...</div>}>
      <HomeContent />
    </Suspense>
  );
}
