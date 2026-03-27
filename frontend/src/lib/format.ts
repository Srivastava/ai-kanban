/**
 * Canonical number formatting utilities — single source of truth.
 * Import from here; do NOT define local copies in components.
 */

/** Format a token count: 1.5M, 2.3K, or raw integer */
export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000)     return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

/** Format a USD cost value */
export function formatCost(n: number | null): string {
  if (n === null) return '—';
  if (n < 0.01)  return `$${n.toFixed(4)}`;
  return `$${n.toFixed(2)}`;
}

/** Format a duration in seconds to a human-readable string */
export function formatDuration(secs: number): string {
  if (secs >= 3_600) {
    const h = Math.floor(secs / 3_600);
    const m = Math.floor((secs % 3_600) / 60);
    return m > 0 ? `${h}h ${m}m` : `${h}h`;
  }
  if (secs >= 60) {
    const m = Math.floor(secs / 60);
    const s = Math.round(secs % 60);
    return s > 0 ? `${m}m ${s}s` : `${m}m`;
  }
  return `${Math.round(secs)}s`;
}
