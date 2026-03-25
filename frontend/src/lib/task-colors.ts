import type { Stage } from '@/types/task';

export const stageColors: Record<Stage, string> = {
  backlog: 'bg-slate-500',
  planning: 'bg-blue-500',
  ready: 'bg-amber-500',
  in_progress: 'bg-orange-500',
  review: 'bg-purple-500',
  done: 'bg-green-500',
};

export const stageBorderColors: Record<Stage, string> = {
  backlog: 'border-l-4 border-l-slate-400',
  planning: 'border-l-4 border-l-blue-500',
  ready: 'border-l-4 border-l-amber-500',
  in_progress: 'border-l-4 border-l-orange-500',
  review: 'border-l-4 border-l-purple-500',
  done: 'border-l-4 border-l-green-500',
};

export const stageHeaderBorder: Record<Stage, string> = {
  backlog: 'border-t-slate-400',
  planning: 'border-t-blue-500',
  ready: 'border-t-amber-500',
  in_progress: 'border-t-orange-500',
  review: 'border-t-purple-500',
  done: 'border-t-green-500',
};

export const stageTextColor: Record<Stage, string> = {
  backlog: 'text-slate-500 dark:text-slate-400',
  planning: 'text-blue-600 dark:text-blue-400',
  ready: 'text-amber-600 dark:text-amber-400',
  in_progress: 'text-orange-600 dark:text-orange-400',
  review: 'text-purple-600 dark:text-purple-400',
  done: 'text-green-600 dark:text-green-400',
};

export const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

export const priorityConfig: Record<number, { label: string; className: string }> = {
  1: { label: 'Low',      className: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300' },
  2: { label: 'Medium',   className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300' },
  3: { label: 'High',     className: 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300' },
  4: { label: 'Critical', className: 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300' },
};
