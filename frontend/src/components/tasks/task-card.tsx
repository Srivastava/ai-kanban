'use client';

import { useState } from 'react';
import Link from 'next/link';
import { Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ConfirmDeleteDialog } from './confirm-delete-dialog';
import { useDeleteTask } from '@/hooks/use-tasks';
import type { Task, Stage } from '@/types/task';

const stageColors: Record<Stage, string> = {
  backlog: 'bg-gray-500',
  planning: 'bg-blue-500',
  ready: 'bg-yellow-500',
  in_progress: 'bg-orange-500',
  review: 'bg-purple-500',
  done: 'bg-green-500',
};

const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

interface TaskCardProps {
  task: Task;
}

export function TaskCard({ task }: TaskCardProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();
  const createdDate = new Date(task.created_at).toLocaleDateString();

  return (
    <div className="relative group">
      <Link href={`/tasks/${task.id}`}>
        <Card className="hover:shadow-md transition-shadow cursor-pointer">
          <CardHeader className="pb-2">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-base font-medium line-clamp-2 pr-6">
                {task.title}
              </CardTitle>
              <Badge className={`${stageColors[task.stage]} text-white shrink-0`}>
                {stageLabels[task.stage]}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            {task.description && (
              <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
                {task.description}
              </p>
            )}
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>Priority: {task.priority}</span>
              <span>{createdDate}</span>
            </div>
          </CardContent>
        </Card>
      </Link>

      {/* Trash button floats over top-right, outside the Link */}
      <Button
        variant="ghost"
        size="icon"
        aria-label="Delete task"
        className="absolute top-2 right-2 h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
        onClick={(e) => { e.preventDefault(); e.stopPropagation(); setConfirmOpen(true); }}
      >
        <Trash2 className="h-3.5 w-3.5" />
      </Button>

      <ConfirmDeleteDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={task.title}
        onConfirm={() => deleteTask(task.id, { onError: () => setConfirmOpen(false) })}
        isDeleting={isDeleting}
      />
    </div>
  );
}
