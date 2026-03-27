import type { Stage } from '@/types/task';

/**
 * Stage color tokens — all values derive from CSS custom properties
 * defined in globals.css and registered in @theme inline.
 * Change colors in one place: src/app/globals.css.
 */

/** Solid badge background + white text (kanban column tabs, list badges) */
export const stageColors: Record<Stage, string> = {
  backlog:     'bg-stage-backlog',
  planning:    'bg-stage-planning',
  ready:       'bg-stage-ready',
  in_progress: 'bg-stage-in-progress',
  review:      'bg-stage-review',
  done:        'bg-stage-done',
};

/** Left border accent on task cards in list view */
export const stageBorderColors: Record<Stage, string> = {
  backlog:     'border-l-4 border-l-stage-backlog',
  planning:    'border-l-4 border-l-stage-planning',
  ready:       'border-l-4 border-l-stage-ready',
  in_progress: 'border-l-4 border-l-stage-in-progress',
  review:      'border-l-4 border-l-stage-review',
  done:        'border-l-4 border-l-stage-done',
};

/** Top border accent on kanban columns */
export const stageHeaderBorder: Record<Stage, string> = {
  backlog:     'border-t-stage-backlog',
  planning:    'border-t-stage-planning',
  ready:       'border-t-stage-ready',
  in_progress: 'border-t-stage-in-progress',
  review:      'border-t-stage-review',
  done:        'border-t-stage-done',
};

/** Text color — theme-aware (dark in light mode, light in dark mode) */
export const stageTextColor: Record<Stage, string> = {
  backlog:     'text-stage-backlog-text',
  planning:    'text-stage-planning-text',
  ready:       'text-stage-ready-text',
  in_progress: 'text-stage-in-progress-text',
  review:      'text-stage-review-text',
  done:        'text-stage-done-text',
};

/** Ambient card background tint — hue-matched to each stage, light/dark aware */
export const stageCardBg: Record<Stage, string> = {
  backlog:     'bg-card-tint-backlog',
  planning:    'bg-card-tint-planning',
  ready:       'bg-card-tint-ready',
  in_progress: 'bg-card-tint-in-progress',
  review:      'bg-card-tint-review',
  done:        'bg-card-tint-done',
};

/** Dashed border for empty column states — stage-hued, subtle opacity */
export const stageEmptyBorder: Record<Stage, string> = {
  backlog:     'border-stage-backlog/30 hover:border-stage-backlog/60',
  planning:    'border-stage-planning/30 hover:border-stage-planning/60',
  ready:       'border-stage-ready/30 hover:border-stage-ready/60',
  in_progress: 'border-stage-in-progress/30 hover:border-stage-in-progress/60',
  review:      'border-stage-review/30 hover:border-stage-review/60',
  done:        'border-stage-done/30 hover:border-stage-done/60',
};

/** Human-readable stage names */
export const stageLabels: Record<Stage, string> = {
  backlog:     'Backlog',
  planning:    'Planning',
  ready:       'Ready',
  in_progress: 'In Progress',
  review:      'Review',
  done:        'Done',
};

/**
 * Subtle chip config — muted background + themed text + soft border.
 * Uses opacity modifiers on the solid stage color for background/border.
 */
export const stageChipConfig: Record<Stage, { label: string; className: string }> = {
  backlog:     { label: 'Backlog',     className: 'bg-stage-backlog/15 text-stage-backlog-text border-stage-backlog/25' },
  planning:    { label: 'Planning',    className: 'bg-stage-planning/15 text-stage-planning-text border-stage-planning/25' },
  ready:       { label: 'Ready',       className: 'bg-stage-ready/15 text-stage-ready-text border-stage-ready/25' },
  in_progress: { label: 'In Progress', className: 'bg-stage-in-progress/15 text-stage-in-progress-text border-stage-in-progress/25' },
  review:      { label: 'Review',      className: 'bg-stage-review/15 text-stage-review-text border-stage-review/25' },
  done:        { label: 'Done',        className: 'bg-stage-done/15 text-stage-done-text border-stage-done/25' },
};

/** Priority badge config */
export const priorityConfig: Record<number, { label: string; className: string }> = {
  1: { label: 'Low',      className: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300' },
  2: { label: 'Medium',   className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300' },
  3: { label: 'High',     className: 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300' },
  4: { label: 'Critical', className: 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300' },
};
