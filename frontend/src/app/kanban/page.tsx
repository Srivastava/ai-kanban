'use client';

import { Suspense, useState } from 'react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { KanbanBoardSkeleton } from '@/components/kanban/kanban-board-skeleton';
import { useTasks } from '@/hooks/use-tasks';
import { Button } from '@/components/ui/button';
import { Plus, RefreshCw } from 'lucide-react';
import { CreateTaskDialog } from '@/components/tasks/create-task-dialog';
import { Sidebar } from '@/components/layout/sidebar';
import type { Stage } from '@/types/task';
import { useSidebarMetrics } from '@/hooks/use-sidebar-metrics';

function MetricsStrip() {
  const metrics = useSidebarMetrics();
  if (!metrics) return null;
  return (
    <div className="-mx-4 sm:-mx-6 px-4 sm:px-6 mb-4 overflow-x-auto border-b border-border pb-4">
      <div className="flex gap-3 min-w-max">
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

interface KanbanContentProps {
  onCreateTask: (stage: Stage) => void;
}

function KanbanContent({ onCreateTask }: KanbanContentProps) {
  const { data: tasks = [], isLoading, error, refetch } = useTasks();

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-3 text-center">
        <p className="text-sm font-medium text-destructive">Couldn't load tasks.</p>
        <p className="text-xs text-muted-foreground max-w-[240px]">
          The board is having a moment. Check your connection and try again.
        </p>
        <Button variant="outline" size="sm" className="gap-1.5" onClick={() => refetch()}>
          <RefreshCw className="h-3.5 w-3.5" />
          Retry
        </Button>
      </div>
    );
  }

  return <KanbanBoard tasks={tasks} isLoading={isLoading} onCreateTask={onCreateTask} />;
}

export default function KanbanPage() {
  const [createOpen, setCreateOpen] = useState(false);

  const handleCreateTask = (_stage: Stage) => {
    // Open the create dialog; tasks default to backlog and can be dragged to the target stage
    setCreateOpen(true);
  };

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <div className="border-b border-border px-4 sm:px-6 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-black tracking-tighter leading-none">Kanban</h1>
            <p className="text-xs text-stage-in-progress-text mt-0.5 font-medium">Drag tasks across stages</p>
          </div>
          <Button onClick={() => setCreateOpen(true)} size="sm">
            <Plus className="mr-1.5 h-4 w-4" />
            New Task
          </Button>
        </div>
        <main className="flex-1 p-4 sm:p-6 pb-20 md:pb-6 overflow-x-auto">
          <MetricsStrip />
          <Suspense fallback={<KanbanBoardSkeleton />}>
            <KanbanContent onCreateTask={handleCreateTask} />
          </Suspense>
        </main>
      </div>
      <CreateTaskDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}
