'use client';

import { useState } from 'react';
import {
  DndContext,
  DragEndEvent,
  DragOverlay,
  DragStartEvent,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import { KanbanColumn } from './kanban-column';
import { KanbanCard } from './kanban-card';
import { useUpdateTask } from '@/hooks/use-tasks';
import type { Task, Stage } from '@/types/task';

const stages: Stage[] = ['backlog', 'planning', 'ready', 'in_progress', 'review', 'done'];

interface KanbanBoardProps {
  tasks: Task[];
  isLoading?: boolean;
}

export function KanbanBoard({ tasks, isLoading }: KanbanBoardProps) {
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const updateTask = useUpdateTask();

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  );

  const tasksByStage = stages.reduce((acc, stage) => {
    acc[stage] = tasks.filter((task) => task.stage === stage);
    return acc;
  }, {} as Record<Stage, Task[]>);

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
    const newStage = over.id as Stage;

    const task = tasks.find((t) => t.id === taskId);
    if (task && task.stage !== newStage) {
      updateTask.mutate({
        id: taskId,
        data: { stage: newStage },
      });
    }
  };

  return (
    <DndContext
      sensors={sensors}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div className="flex gap-4 overflow-x-auto pb-4">
        {stages.map((stage) => (
          <KanbanColumn
            key={stage}
            stage={stage}
            tasks={tasksByStage[stage]}
            isLoading={isLoading}
          />
        ))}
      </div>
      <DragOverlay>
        {activeTask ? <KanbanCard task={activeTask} /> : null}
      </DragOverlay>
    </DndContext>
  );
}
