'use client';

import { useState } from 'react';
import Link from 'next/link';
import { Trash2 } from 'lucide-react';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ConfirmDeleteDialog } from '@/components/tasks/confirm-delete-dialog';
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

interface KanbanCardProps {
  task: Task;
  isOverlay?: boolean;
}

export function KanbanCard({ task, isOverlay = false }: KanbanCardProps) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const { mutate: deleteTask, isPending: isDeleting } = useDeleteTask();

  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: task.id });

  const style = { transform: CSS.Transform.toString(transform), transition };

  return (
    <div ref={setNodeRef} style={style} className={`relative group ${isDragging ? 'opacity-50' : ''}`}>
      {/* Drag handle wraps only the card content */}
      <div {...attributes} {...listeners}>
        <Link href={`/tasks/${task.id}`}>
          <Card className="cursor-pointer hover:shadow-md transition-shadow">
            <CardHeader className="p-3 pb-1">
              <div className="flex items-start justify-between gap-2">
                <CardTitle className="text-sm font-medium line-clamp-2 pr-5">
                  {task.title}
                </CardTitle>
                <Badge className={`${stageColors[task.stage]} text-white text-[10px] px-1.5`}>
                  {task.priority}
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="p-3 pt-0">
              {task.description && (
                <p className="text-xs text-muted-foreground line-clamp-2">
                  {task.description}
                </p>
              )}
            </CardContent>
          </Card>
        </Link>
      </div>

      {!isOverlay && (
        <>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Delete task"
            className="absolute top-1.5 right-1.5 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity text-destructive hover:bg-destructive/10 z-10"
            onClick={(e) => { e.preventDefault(); e.stopPropagation(); setConfirmOpen(true); }}
          >
            <Trash2 className="h-3 w-3" />
          </Button>

          <ConfirmDeleteDialog
            open={confirmOpen}
            onOpenChange={setConfirmOpen}
            title={task.title}
            onConfirm={() => deleteTask(task.id, { onError: () => setConfirmOpen(false) })}
            isDeleting={isDeleting}
          />
        </>
      )}
    </div>
  );
}
