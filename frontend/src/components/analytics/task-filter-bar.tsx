'use client';

import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';

interface TaskOption {
  id: string;
  title: string;
}

interface Props {
  selectedTaskId: string | null;
  onSelect: (taskId: string | null) => void;
}

export function TaskFilterBar({ selectedTaskId, onSelect }: Props) {
  const { data: tasks = [] } = useQuery({
    queryKey: ['tasks', 'for-filter'],
    queryFn: () => apiClient<TaskOption[]>('/api/tasks'),
    select: (data: any[]) => data
      .map((t: any) => ({ id: t.id, title: t.title }))
      .slice(0, 50), // cap for dropdown size
  });

  return (
    <div className="sticky top-0 z-10 border-b border-border bg-background/95 backdrop-blur px-4 sm:px-6 py-3 flex items-center gap-3">
      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider shrink-0">
        Filter by task
      </span>
      <select
        value={selectedTaskId ?? ''}
        onChange={(e) => onSelect(e.target.value || null)}
        className="flex-1 max-w-xs rounded-md border border-border bg-card px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
      >
        <option value="">All Tasks</option>
        {tasks.map((t) => (
          <option key={t.id} value={t.id}>{t.title}</option>
        ))}
      </select>
      {selectedTaskId && (
        <button
          onClick={() => onSelect(null)}
          className="text-xs text-muted-foreground hover:text-foreground underline"
        >
          Clear
        </button>
      )}
    </div>
  );
}
