/**
 * Shared chart color constants — all referencing CSS design tokens.
 *
 * Chart tokens (--chart-1..5) are aligned with stage color identity:
 *   chart-1 = planning (blue)
 *   chart-2 = done (green)
 *   chart-3 = in_progress (orange)
 *   chart-4 = review (purple)
 *   chart-5 = ready (amber)
 *
 * Use `var(--chart-N)` directly — these are full OKLCH color values
 * in globals.css, so NO hsl() wrapper is needed or correct.
 */

/** Cycling series palette for multi-series charts (tools, languages, etc.) */
export const SERIES_COLORS = [
  'var(--chart-1)',
  'var(--chart-2)',
  'var(--chart-3)',
  'var(--chart-4)',
  'var(--chart-5)',
] as const;

/** Semantic token series: input tokens, output tokens, cached tokens */
export const TOKEN_COLORS = {
  input:  'var(--chart-1)',  // blue — new data going in
  output: 'var(--chart-4)',  // purple — generated output
  cached: 'var(--chart-5)',  // amber — efficient cached reads
} as const;

/** Semantic colors for session/task status */
export const STATUS_COLORS: Record<string, string> = {
  completed: 'var(--chart-2)',              // green = done
  running:   'var(--chart-1)',              // blue = active/in-flight
  pending:   'var(--chart-5)',              // amber = waiting/ready
  stopped:   'var(--muted-foreground)',     // neutral
  failed:    'var(--destructive)',          // red = error
};

/** Return a cycling series color by index */
export function seriesColor(index: number): string {
  return SERIES_COLORS[index % SERIES_COLORS.length];
}

/** Return the semantic color for a session status string */
export function statusColor(status: string): string {
  return STATUS_COLORS[status] ?? 'var(--muted-foreground)';
}
