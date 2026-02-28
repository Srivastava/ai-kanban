'use client';

import Link from 'next/link';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
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
}

export function KanbanCard({ task }: KanbanCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: task.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      className={isDragging ? 'opacity-50' : ''}
    >
      <Link href={`/tasks/${task.id}`}>
        <Card className="cursor-pointer hover:shadow-md transition-shadow">
          <CardHeader className="p-3 pb-1">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-sm font-medium line-clamp-2">
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
  );
}
