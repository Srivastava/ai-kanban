'use client';

import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import {
  Plus, AlertTriangle,
  Inbox, Lightbulb, Rocket, Zap, Search, Trophy,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { KanbanCard } from './kanban-card';
import { KanbanCardSkeleton } from './kanban-card-skeleton';
import type { Task, Stage } from '@/types/task';
import type { CostByTask } from '@/types/analytics';
import { stageLabels, stageHeaderBorder, stageTextColor, stageCardBg, stageEmptyBorder } from '@/lib/task-colors';

interface StageEmptyConfig {
  icon: LucideIcon;
  headline: string;
  subtext: string;
}

const stageEmptyConfig: Record<Stage, StageEmptyConfig> = {
  backlog:     { icon: Inbox,     headline: 'The queue is empty.',         subtext: 'Feed it something to chew on.' },
  planning:    { icon: Lightbulb, headline: 'No active blueprints.',       subtext: 'An AI agent is waiting to plan.' },
  ready:       { icon: Rocket,    headline: 'Launch pad is clear.',        subtext: 'Nothing queued for the sprint.' },
  in_progress: { icon: Zap,       headline: 'All quiet on this front.',    subtext: 'No tasks actively running.' },
  review:      { icon: Search,    headline: 'Nothing under the glass.',    subtext: 'Either pristine or unstarted.' },
  done:        { icon: Trophy,    headline: 'No wins logged yet.',         subtext: 'Ship something worth celebrating.' },
};

// ── Stage empty state ──────────────────────────────────────────────────────

interface StageEmptyStateProps {
  stage: Stage;
  isOver: boolean;
  onAddTask: () => void;
}

function StageEmptyState({ stage, isOver, onAddTask }: StageEmptyStateProps) {
  const { icon: Icon, headline, subtext } = stageEmptyConfig[stage];
  const borderClass = stageEmptyBorder[stage];
  const textClass = stageTextColor[stage];

  const isOverClass = 'scale-[1.02] bg-muted/60 ring-1 ring-primary/20 border-primary/40';

  return (
    <button
      className={[
        'w-full min-h-[160px] flex flex-col items-center justify-center gap-3 rounded-md',
        'border-2 border-dashed transition-all duration-200 animate-fade-in-up',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50',
        'group/empty cursor-pointer',
        isOver ? isOverClass : borderClass,
      ].join(' ')}
      onClick={onAddTask}
      aria-label={`Add task to ${stageLabels[stage]}`}
    >
      <div className={`${textClass} opacity-50 group-hover/empty:opacity-80 group-focus-visible/empty:opacity-80 transition-opacity motion-safe:animate-float`}>
        <Icon className="h-7 w-7" strokeWidth={1.5} />
      </div>
      <div className="flex flex-col items-center gap-1 text-center px-4">
        <span className={`text-xs font-semibold ${textClass} opacity-60 group-hover/empty:opacity-90 group-focus-visible/empty:opacity-90 transition-opacity`}>
          {headline}
        </span>
        <span className="text-[11px] text-muted-foreground/50 group-hover/empty:text-muted-foreground/70 group-focus-visible/empty:text-muted-foreground/70 transition-colors">
          {subtext}
        </span>
      </div>
      <div className={`flex items-center gap-1.5 text-[11px] font-medium opacity-0 group-hover/empty:opacity-60 group-focus-visible/empty:opacity-60 transition-opacity duration-150 ${textClass}`}>
        <Plus className="h-3 w-3" />
        Add task
      </div>
    </button>
  );
}

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
      className={`flex flex-col w-full sm:min-w-[270px] sm:max-w-[310px] rounded-lg bg-muted/10 border-t-4 ${stageHeaderBorder[stage]}`}
    >
      {/* Column header — colored zone */}
      <div className={`flex items-center justify-between px-3 pt-3 pb-2.5 ${stageCardBg[stage]}`}>
        <div className="flex items-center gap-2">
          <h2 className={`font-black text-xs uppercase tracking-widest ${stageTextColor[stage]}`}>
            {stageLabels[stage]}
          </h2>
          <span
            key={tasks.length}
            className={`text-xs font-bold tabular-nums ${stageTextColor[stage]} motion-safe:animate-count-bump`}
          >
            {tasks.length}
          </span>
          {isOverWip && (
            <span
              role="alert"
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
            className={`h-9 w-9 ${stageTextColor[stage]} opacity-60 hover:opacity-100 transition-opacity`}
            onClick={() => onCreateTask(stage)}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>

      {/* Drop zone */}
      <div
        ref={setNodeRef}
        className={`flex-1 rounded-lg p-2 space-y-2 min-h-[200px] transition-all duration-200 ${
          isOver ? 'bg-muted/60 ring-1 ring-primary/20 scale-[1.005]' : 'bg-muted/30'
        }`}
      >
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 2 }).map((_, i) => (
              <KanbanCardSkeleton key={i} index={i} />
            ))}
          </div>
        ) : (
          <>
            <SortableContext items={tasks.map((t) => t.id)} strategy={verticalListSortingStrategy}>
              {recentTasks.map((task, i) => (
                <KanbanCard key={task.id} task={task} costData={costByTaskId?.get(task.id)} index={i} />
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
                  {olderTasks.map((task, i) => (
                    <KanbanCard key={task.id} task={task} costData={costByTaskId?.get(task.id)} index={recentTasks.length + i} />
                  ))}
                </>
              )}
            </SortableContext>

            {tasks.length === 0 && (
              <StageEmptyState
                stage={stage}
                isOver={isOver}
                onAddTask={() => onCreateTask?.(stage)}
              />
            )}
          </>
        )}
      </div>
    </div>
  );
}
