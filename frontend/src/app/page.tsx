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
import { useSidebarMetrics } from '@/hooks/use-sidebar-metrics';

// ── mobile metrics strip ──────────────────────────────────────────────────────

function MobileMetricsStrip() {
  const metrics = useSidebarMetrics();
  if (!metrics) return null;
  return (
    <div className="md:hidden -mx-4 px-4 mb-4 overflow-x-auto">
      <div className="flex gap-3 pb-1 min-w-max">
        {metrics.map((m) => (
          <div key={m.label} className="flex flex-col items-center bg-muted/50 rounded-lg px-3 py-1.5 min-w-[64px]">
            <span className="text-[10px] text-muted-foreground whitespace-nowrap">{m.label}</span>
            <span className="text-xs font-semibold tabular-nums">{m.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── stats strip ───────────────────────────────────────────────────────────────

function StatsStrip({ onNewTask }: { onNewTask: () => void }) {
  const { data: activeSessions = [] } = useAllSessions(['running', 'pending']);
  const { data: allTasks = [] } = useTasks();
  const { data: costData = [] } = useQuery<CostByTask[]>({
    queryKey: ['cost-by-task'],
    queryFn: () => apiClient<CostByTask[]>('/api/analytics/cost/by-task'),
    staleTime: 60_000,
  });

  const activeCount = activeSessions.length;
  const inReviewCount = allTasks.filter((t) => t.stage === 'review').length;

  // Today's cost: sum cost_usd from costData — API doesn't filter by date so sum all
  // (the API returns cost by task total; we approximate "today" by summing all available)
  const todayCost = costData.reduce((sum, c) => sum + (c.cost_usd ?? 0), 0);

  const hasStats = activeCount > 0 || inReviewCount > 0 || todayCost > 0;

  return (
    <>
      {/* Stats strip */}
      {hasStats && (
        <div className="mb-5 flex flex-wrap items-center gap-2">
          {activeCount > 0 && (
            <span
              className="text-xs bg-emerald-500/10 border border-emerald-500/20 rounded-lg px-3 py-1.5 flex items-center gap-2 text-emerald-700 dark:text-emerald-400 font-medium motion-safe:animate-fade-in-up"
              style={{ animationDelay: '0ms' }}
            >
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 motion-safe:animate-breathe shrink-0" />
              <strong className="font-black">{activeCount}</strong> running
            </span>
          )}
          {inReviewCount > 0 && (
            <span
              className="text-xs bg-stage-review/10 border border-stage-review/20 rounded-lg px-3 py-1.5 flex items-center gap-2 text-stage-review-text font-medium motion-safe:animate-fade-in-up"
              style={{ animationDelay: '60ms' }}
            >
              <strong className="font-black">{inReviewCount}</strong> in review
            </span>
          )}
          {todayCost > 0 && (
            <span
              className="text-xs bg-amber-500/10 border border-amber-500/20 rounded-lg px-3 py-1.5 flex items-center gap-2 text-amber-700 dark:text-amber-400 font-medium motion-safe:animate-fade-in-up"
              style={{ animationDelay: '120ms' }}
            >
              <strong className="font-black">${todayCost.toFixed(2)}</strong> total cost
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
          <MobileMetricsStrip />
          <StatsStrip onNewTask={() => setDialogOpen(true)} />
          <Suspense fallback={<div role="status" aria-live="polite" className="flex items-center justify-center h-64 text-sm text-muted-foreground">Loading...</div>}>
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
    <Suspense fallback={<div role="status" aria-live="polite" className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">Loading...</div>}>
      <HomeContent />
    </Suspense>
  );
}
