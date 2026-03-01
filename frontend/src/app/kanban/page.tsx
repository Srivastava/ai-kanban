'use client';

import { Suspense, useState } from 'react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { useTasks } from '@/hooks/use-tasks';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import { CreateTaskDialog } from '@/components/tasks/create-task-dialog';

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
    <div className="h-screen bg-background p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Kanban Board</h1>
        <Button onClick={() => setCreateOpen(true)} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          New Task
        </Button>
      </div>
      <Suspense fallback={<div>Loading...</div>}>
        <KanbanContent />
      </Suspense>
      <CreateTaskDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}
