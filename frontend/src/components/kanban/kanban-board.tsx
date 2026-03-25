'use client';

import { useState, useCallback, useEffect } from 'react';
import {
  DndContext,
  DragEndEvent,
  DragOverlay,
  DragStartEvent,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import { Eye, EyeOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { KanbanColumn } from './kanban-column';
import { KanbanCard } from './kanban-card';
import { useQuery } from '@tanstack/react-query';
import { useUpdateTask } from '@/hooks/use-tasks';
import { useAllSessions } from '@/hooks/use-sessions';
import { apiClient } from '@/lib/api-client';
import type { Task, Stage } from '@/types/task';
import type { CostByTask } from '@/types/analytics';
import { stageLabels, stageChipConfig } from '@/lib/task-colors';

const stages: Stage[] = ['backlog', 'planning', 'ready', 'in_progress', 'review', 'done'];

interface UndoEntry {
  taskId: string;
  taskTitle: string;
  fromStage: Stage;
  toStage: Stage;
}

interface KanbanBoardProps {
  tasks: Task[];
  isLoading?: boolean;
  onCreateTask?: (stage: Stage) => void;
}

export function KanbanBoard({ tasks, isLoading, onCreateTask }: KanbanBoardProps) {
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const [hideEmpty, setHideEmpty] = useState(false);
  const [undoEntry, setUndoEntry] = useState<UndoEntry | null>(null);
  const [undoTimer, setUndoTimer] = useState<ReturnType<typeof setTimeout> | null>(null);

  const updateTask = useUpdateTask();
  const { data: activeSessions = [] } = useAllSessions(['running', 'pending']);
  const { data: costData = [] } = useQuery<CostByTask[]>({
    queryKey: ['cost-by-task'],
    queryFn: () => apiClient<CostByTask[]>('/api/analytics/cost/by-task'),
    staleTime: 60_000,
  });
  const costByTaskId = new Map(costData.map((c) => [c.task_id, c]));

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } })
  );

  const tasksByStage = stages.reduce((acc, stage) => {
    acc[stage] = tasks.filter((task) => task.stage === stage);
    return acc;
  }, {} as Record<Stage, Task[]>);

  const visibleStages = hideEmpty
    ? stages.filter((s) => tasksByStage[s].length > 0)
    : stages;

  const totalTasks = tasks.length;
  const activeSessionCount = activeSessions.length;
  const inProgressCount = tasksByStage['in_progress']?.length ?? 0;
  const reviewCount = tasksByStage['review']?.length ?? 0;

  const clearUndo = useCallback(() => {
    setUndoEntry(null);
    setUndoTimer((prev) => {
      if (prev) clearTimeout(prev);
      return null;
    });
  }, []);

  const handleUndo = useCallback(() => {
    if (!undoEntry) return;
    updateTask.mutate({ id: undoEntry.taskId, data: { stage: undoEntry.fromStage } });
    clearUndo();
  }, [undoEntry, updateTask, clearUndo]);

  const handleDragStart = (event: DragStartEvent) => {
    const taskId = event.active.id as string;
    const task = tasks.find((t) => t.id === taskId);
    setActiveTask(task || null);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveTask(null);
    if (!over) return;

    const taskId = active.id as string;
    const task = tasks.find((t) => t.id === taskId);
    if (!task) return;

    // over.id can be a stage name (dropped on column) or a task id (dropped on card)
    const validStages = new Set<string>(stages);
    let newStage: Stage;
    if (validStages.has(over.id as string)) {
      newStage = over.id as Stage;
    } else {
      // dropped over another task — use that task's stage
      const overTask = tasks.find((t) => t.id === over.id);
      if (!overTask) return;
      newStage = overTask.stage;
    }

    if (task.stage === newStage) return;

    const fromStage = task.stage;
    updateTask.mutate({ id: taskId, data: { stage: newStage } });

    // Show undo toast
    if (undoTimer) clearTimeout(undoTimer);
    setUndoEntry({ taskId, taskTitle: task.title, fromStage, toStage: newStage });
    const timer = setTimeout(clearUndo, 5000);
    setUndoTimer(timer);
  };

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (undoTimer) clearTimeout(undoTimer);
    };
  }, [undoTimer]);

  return (
    <div className="space-y-3">
      {/* Board stats bar */}
      <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground pb-1">
        <span>
          <span className="font-medium text-foreground">{totalTasks}</span> tasks
        </span>
        {activeSessionCount > 0 && (
          <span className="flex items-center gap-1.5">
            <span className="inline-block h-2 w-2 rounded-full bg-green-500 motion-safe:animate-breathe" />
            <span className="font-medium text-foreground">{activeSessionCount}</span> active
          </span>
        )}
        {inProgressCount > 0 && (
          <span>
            <span className="font-medium text-foreground">{inProgressCount}</span> in progress
          </span>
        )}
        {reviewCount > 0 && (
          <span>
            <span className="font-medium text-foreground">{reviewCount}</span> in review
          </span>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto h-7 text-xs gap-1"
          onClick={() => setHideEmpty((h) => !h)}
        >
          {hideEmpty ? <Eye className="h-3 w-3" /> : <EyeOff className="h-3 w-3" />}
          {hideEmpty ? 'Show empty' : 'Hide empty'}
        </Button>
      </div>

      <DndContext sensors={sensors} onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
        {/* Desktop: horizontal scroll */}
        <div className="hidden sm:flex gap-3 overflow-x-auto pb-4">
          {visibleStages.map((stage) => (
            <KanbanColumn
              key={stage}
              stage={stage}
              tasks={tasksByStage[stage]}
              isLoading={isLoading}
              onCreateTask={onCreateTask}
              costByTaskId={costByTaskId}
            />
          ))}
        </div>

        {/* Mobile: stage tabs + single column */}
        <MobileKanban
          tasksByStage={tasksByStage}
          isLoading={isLoading}
          onCreateTask={onCreateTask}
          costByTaskId={costByTaskId}
        />

        <DragOverlay>
          {activeTask ? <KanbanCard task={activeTask} isOverlay /> : null}
        </DragOverlay>
      </DndContext>

      {/* Undo toast */}
      {undoEntry && (
        <div className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 rounded-lg border border-border bg-popover px-4 py-2.5 shadow-lg text-sm max-w-[90vw] motion-safe:animate-toast-enter">
          <span className="text-muted-foreground truncate">
            Moved to {stageLabels[undoEntry.toStage]}
          </span>
          <Button variant="outline" size="sm" className="h-7 text-xs shrink-0" onClick={handleUndo}>
            Undo
          </Button>
          <Button variant="ghost" size="sm" className="h-7 text-xs shrink-0" onClick={clearUndo}>
            Dismiss
          </Button>
        </div>
      )}
    </div>
  );
}

// ── Mobile kanban ─────────────────────────────────────────────────────────────

interface MobileKanbanProps {
  tasksByStage: Record<Stage, Task[]>;
  isLoading?: boolean;
  onCreateTask?: (stage: Stage) => void;
  costByTaskId?: Map<string, CostByTask>;
}

function MobileKanban({ tasksByStage, isLoading, onCreateTask, costByTaskId }: MobileKanbanProps) {
  const [activeStage, setActiveStage] = useState<Stage>('in_progress');

  return (
    <div className="sm:hidden">
      {/* Stage tab strip */}
      <div className="flex overflow-x-auto gap-1 pb-2 mb-3 border-b border-border">
        {(Object.keys(stageLabels) as Stage[]).map((stage) => (
          <button
            key={stage}
            onClick={() => setActiveStage(stage)}
            className={`shrink-0 flex items-center gap-1.5 px-3 py-2 rounded-full text-xs font-medium transition-colors min-h-[36px] ${
              activeStage === stage
                ? `${stageChipConfig[stage].className} border`
                : 'text-muted-foreground hover:bg-muted'
            }`}
          >
            {stageLabels[stage]}
            <span
              className={`text-[10px] ${activeStage === stage ? 'opacity-80' : 'opacity-60'}`}
            >
              {tasksByStage[stage].length}
            </span>
          </button>
        ))}
      </div>

      {/* Single active column — key remounts on stage change to trigger card stagger */}
      <KanbanColumn
        key={activeStage}
        stage={activeStage}
        tasks={tasksByStage[activeStage]}
        isLoading={isLoading}
        onCreateTask={onCreateTask}
        costByTaskId={costByTaskId}
      />
    </div>
  );
}

