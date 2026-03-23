'use client';

import { Suspense, useState } from 'react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { useTasks } from '@/hooks/use-tasks';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
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
  const { data: tasks = [], isLoading, error } = useTasks();

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-destructive">Error loading tasks: {error.message}</p>
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
          <h1 className="text-xl font-semibold">Kanban Board</h1>
          <Button onClick={() => setCreateOpen(true)} size="sm">
            <Plus className="mr-2 h-4 w-4" />
            New Task
          </Button>
        </div>
        <main className="flex-1 p-4 sm:p-6 pb-20 md:pb-6 overflow-x-auto">
          <MetricsStrip />
          <Suspense fallback={<div>Loading...</div>}>
            <KanbanContent onCreateTask={handleCreateTask} />
          </Suspense>
        </main>
      </div>
      <CreateTaskDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}
