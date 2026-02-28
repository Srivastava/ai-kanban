'use client';

import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { KanbanCard } from './kanban-card';
import type { Task, Stage } from '@/types/task';

const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

interface KanbanColumnProps {
  stage: Stage;
  tasks: Task[];
}

export function KanbanColumn({ stage, tasks }: KanbanColumnProps) {
  const { setNodeRef, isOver } = useDroppable({
    id: stage,
  });

  return (
    <div className="flex flex-col min-w-[280px] max-w-[320px]">
      <div className="flex items-center justify-between mb-3 px-1">
        <h2 className="font-semibold text-sm">{stageLabels[stage]}</h2>
        <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
          {tasks.length}
        </span>
      </div>
      <div
        ref={setNodeRef}
        className={`flex-1 bg-muted/30 rounded-lg p-2 space-y-2 min-h-[200px] transition-colors ${
          isOver ? 'bg-muted/50' : ''
        }`}
      >
        <SortableContext items={tasks.map(t => t.id)} strategy={verticalListSortingStrategy}>
          {tasks.map((task) => (
            <KanbanCard key={task.id} task={task} />
          ))}
        </SortableContext>
        {tasks.length === 0 && (
          <div className="text-center text-muted-foreground text-sm py-8">
            No tasks
          </div>
        )}
      </div>
    </div>
  );
}
