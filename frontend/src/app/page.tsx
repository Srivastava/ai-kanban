'use client';

import { useState } from 'react';
import { useSearchParams } from 'next/navigation';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { TaskList } from '@/components/tasks/task-list';
import { CreateTaskDialog } from '@/components/tasks/create-task-dialog';
import { useTasks } from '@/hooks/use-tasks';
import type { Stage } from '@/types/task';

export default function Home() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const searchParams = useSearchParams();
  const stageParam = searchParams.get('stage');
  const stage = stageParam && stageParam !== 'all' ? (stageParam as Stage) : undefined;

  const { data: tasks = [], isLoading, error } = useTasks(stage);

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <Header onNewTask={() => setDialogOpen(true)} />
        <main className="flex-1 p-6">
          {error ? (
            <div className="flex items-center justify-center h-64">
              <p className="text-destructive">
                Error loading tasks: {error.message}
              </p>
            </div>
          ) : (
            <TaskList tasks={tasks} isLoading={isLoading} />
          )}
        </main>
      </div>
      <CreateTaskDialog open={dialogOpen} onOpenChange={setDialogOpen} />
    </div>
  );
}
