'use client';

import { useParams, useRouter } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';
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
    deleteTask(taskId, {
      onSuccess: () => router.push('/'),
    });
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-background">
        <header className="border-b border-border px-6 py-4">
          <Button variant="ghost" onClick={() => router.back()}>
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back
          </Button>
        </header>
        <main className="max-w-4xl mx-auto p-6">
          <TaskDetailSkeleton />
        </main>
      </div>
    );
  }

  if (error || !task) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <p className="text-destructive">Task not found</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b border-border px-6 py-4">
        <Button variant="ghost" onClick={() => router.back()}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back
        </Button>
      </header>
      <main className="max-w-4xl mx-auto p-6">
        <TaskDetail task={task} onDelete={handleDelete} isDeleting={isDeleting} />
      </main>
    </div>
  );
}
