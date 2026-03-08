'use client';

import { Suspense, useState } from 'react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { useTasks } from '@/hooks/use-tasks';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import { CreateTaskDialog } from '@/components/tasks/create-task-dialog';
import { Sidebar } from '@/components/layout/sidebar';

function KanbanContent() {
  const { data: tasks = [], isLoading, error } = useTasks();

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-destructive">Error loading tasks: {error.message}</p>
      </div>
    );
  }

  return <KanbanBoard tasks={tasks} isLoading={isLoading} />;
}

export default function KanbanPage() {
  const [createOpen, setCreateOpen] = useState(false);

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
          <Suspense fallback={<div>Loading...</div>}>
            <KanbanContent />
          </Suspense>
        </main>
      </div>
      <CreateTaskDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}
