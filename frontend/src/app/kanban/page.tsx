'use client';

import { Suspense } from 'react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { useTasks } from '@/hooks/use-tasks';

function KanbanContent() {
  const { data: tasks = [], isLoading, error } = useTasks();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-muted-foreground">Loading tasks...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-destructive">Error loading tasks: {error.message}</p>
      </div>
    );
  }

  return <KanbanBoard tasks={tasks} />;
}

export default function KanbanPage() {
  return (
    <div className="h-screen bg-background p-6">
      <h1 className="text-2xl font-bold mb-6">Kanban Board</h1>
      <Suspense fallback={<div>Loading...</div>}>
        <KanbanContent />
      </Suspense>
    </div>
  );
}
