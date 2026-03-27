'use client';

import { useContextUsage } from '@/hooks/use-analytics';

function formatTokens(n: number) {
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

export function ContextWindowGauges() {
  const { data: sessions = [], isLoading } = useContextUsage();

  if (isLoading) {
    return <div className="h-6 bg-muted rounded animate-shimmer" />;
  }

  if (sessions.length === 0) {
    return (
      <p className="text-xs text-muted-foreground italic">
        No active sessions — context gauges appear when Claude is running
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {sessions.map((s) => {
        const pct = s.pct_used;
        const color = pct < 60 ? 'bg-stage-done' : pct < 85 ? 'bg-stage-ready' : 'bg-destructive';
        const textColor = pct < 60 ? 'text-stage-done-text' : pct < 85 ? 'text-stage-ready-text' : 'text-destructive';
        return (
          <div key={s.session_id} className="space-y-1">
            <div className="flex items-center justify-between text-xs">
              <span className="text-muted-foreground truncate max-w-[60%]">{s.task_title}</span>
              <span className={textColor}>
                {formatTokens(s.tokens_in_window)} / {formatTokens(s.context_limit)} ctx
              </span>
            </div>
            <div className="h-1.5 w-full rounded-full bg-muted overflow-hidden">
              <div
                className={`h-full rounded-full ${color}`}
                style={{ width: `${Math.min(pct, 100)}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}
