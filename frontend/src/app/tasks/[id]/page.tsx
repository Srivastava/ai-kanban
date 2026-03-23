'use client';

import { useParams, useRouter } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Sidebar } from '@/components/layout/sidebar';
import { useTask, useDeleteTask } from '@/hooks/use-tasks';
import { TaskDetail } from '@/components/tasks/task-detail';
import { TaskDetailSkeleton } from '@/components/tasks/task-detail-skeleton';

export default function TaskDetailPage() {
  const params = useParams();
  const router = useRouter();
  const taskId = params.id as string;

  const { data: task, isLoading, error } = useTask(taskId);
  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();

  const handleDelete = () => {
    deleteTask(taskId, { onSuccess: () => router.push('/') });
  };

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 min-w-0 overflow-x-hidden flex flex-col">
        <header className="sticky top-0 z-10 border-b border-border bg-background/95 backdrop-blur-sm px-4 sm:px-6 py-3 flex items-center gap-3">
          <Button variant="ghost" size="sm" onClick={() => router.back()} className="-ml-1">
            <ArrowLeft className="mr-1.5 h-4 w-4" />
            <span className="hidden sm:inline">Back</span>
          </Button>
          {task && (
            <h2 className="text-sm font-medium text-muted-foreground truncate">{task.title}</h2>
          )}
        </header>
        <main className="flex-1 px-4 sm:px-6 py-6 pb-20 md:pb-10 max-w-3xl w-full">
          {isLoading ? (
            <TaskDetailSkeleton />
          ) : error || !task ? (
            <div className="flex items-center justify-center py-24">
              <p className="text-destructive">Task not found</p>
            </div>
          ) : (
            <TaskDetail task={task} onDelete={handleDelete} isDeleting={isDeleting} />
          )}
        </main>
      </div>
    </div>
  );
}
