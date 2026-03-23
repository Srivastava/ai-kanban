'use client';

import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { Plus, AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { KanbanCard } from './kanban-card';
import { KanbanCardSkeleton } from './kanban-card-skeleton';
import type { Task, Stage } from '@/types/task';
import type { CostByTask } from '@/types/analytics';

const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

const stageHeaderBorder: Record<Stage, string> = {
  backlog: 'border-t-slate-400',
  planning: 'border-t-blue-500',
  ready: 'border-t-amber-500',
  in_progress: 'border-t-orange-500',
  review: 'border-t-purple-500',
  done: 'border-t-green-500',
};

const stageTextColor: Record<Stage, string> = {
  backlog: 'text-slate-500 dark:text-slate-400',
  planning: 'text-blue-600 dark:text-blue-400',
  ready: 'text-amber-600 dark:text-amber-400',
  in_progress: 'text-orange-600 dark:text-orange-400',
  review: 'text-purple-600 dark:text-purple-400',
  done: 'text-green-600 dark:text-green-400',
};

// WIP limits — warn when column exceeds these
const WIP_LIMITS: Partial<Record<Stage, number>> = {
  in_progress: 3,
  review: 4,
};

const RECENT_DAYS = 7;

interface KanbanColumnProps {
  stage: Stage;
  tasks: Task[];
  isLoading?: boolean;
  onCreateTask?: (stage: Stage) => void;
  costByTaskId?: Map<string, CostByTask>;
}

export function KanbanColumn({ stage, tasks, isLoading, onCreateTask, costByTaskId }: KanbanColumnProps) {
  const { setNodeRef, isOver } = useDroppable({ id: stage });

  const wipLimit = WIP_LIMITS[stage];
  const isOverWip = wipLimit !== undefined && tasks.length > wipLimit;

  // Done column: split into recent vs older
  const isDoneColumn = stage === 'done';
  const cutoff = new Date(Date.now() - RECENT_DAYS * 24 * 60 * 60 * 1000);
  const recentTasks = isDoneColumn ? tasks.filter((t) => new Date(t.updated_at) >= cutoff) : tasks;
  const olderTasks = isDoneColumn ? tasks.filter((t) => new Date(t.updated_at) < cutoff) : [];

  return (
    <div
      className={`flex flex-col min-w-[270px] max-w-[310px] rounded-lg bg-muted/10 border-t-4 ${stageHeaderBorder[stage]}`}
    >
      {/* Column header */}
      <div className="flex items-center justify-between px-2 pt-2.5 pb-2">
        <div className="flex items-center gap-1.5">
          <h2 className={`font-semibold text-sm ${stageTextColor[stage]}`}>
            {stageLabels[stage]}
          </h2>
          <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
            {tasks.length}
          </span>
          {isOverWip && (
            <span
              className="flex items-center gap-0.5 text-[10px] text-amber-600 font-medium"
              title={`WIP limit is ${wipLimit}`}
            >
              <AlertTriangle className="h-3 w-3" />
              WIP
            </span>
          )}
        </div>
        {onCreateTask && (
          <Button
            variant="ghost"
            size="icon"
            aria-label={`Add task to ${stageLabels[stage]}`}
            className="h-6 w-6 text-muted-foreground hover:text-foreground"
            onClick={() => onCreateTask(stage)}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>

      {/* Drop zone */}
      <div
        ref={setNodeRef}
        className={`flex-1 rounded-lg p-2 space-y-2 min-h-[200px] transition-colors ${
          isOver ? 'bg-muted/60 ring-1 ring-primary/20' : 'bg-muted/30'
        }`}
      >
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 2 }).map((_, i) => (
              <KanbanCardSkeleton key={i} />
            ))}
          </div>
        ) : (
          <>
            <SortableContext items={tasks.map((t) => t.id)} strategy={verticalListSortingStrategy}>
              {recentTasks.map((task) => (
                <KanbanCard key={task.id} task={task} costData={costByTaskId?.get(task.id)} />
              ))}

              {isDoneColumn && olderTasks.length > 0 && (
                <>
                  <div className="flex items-center gap-2 py-1">
                    <div className="flex-1 h-px bg-border" />
                    <span className="text-[10px] text-muted-foreground whitespace-nowrap">
                      Older than {RECENT_DAYS}d
                    </span>
                    <div className="flex-1 h-px bg-border" />
                  </div>
                  {olderTasks.map((task) => (
                    <KanbanCard key={task.id} task={task} costData={costByTaskId?.get(task.id)} />
                  ))}
                </>
              )}
            </SortableContext>

            {tasks.length === 0 && (
              <button
                className="w-full min-h-[140px] flex flex-col items-center justify-center gap-2 rounded-md border-2 border-dashed border-border/50 text-muted-foreground hover:border-primary/40 hover:text-primary/70 transition-colors"
                onClick={() => onCreateTask?.(stage)}
              >
                <Plus className="h-4 w-4" />
                <span className="text-xs">Add task</span>
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}
