'use client';

import { useState } from 'react';
import { useTokensByTask } from '@/hooks/use-analytics';
import { estimateCost } from '@/lib/pricing';
import { seriesColor } from '@/lib/chart-colors';
import { formatTokens as fmt } from '@/lib/format';

interface Props {
  onTaskSelect?: (taskId: string) => void;
  selectedTaskId?: string | null;
}

export function ProjectBubbleChart({ onTaskSelect, selectedTaskId }: Props) {
  const { data: tasks = [], isLoading } = useTokensByTask();
  const [hoverId, setHoverId] = useState<string | null>(null);

  const active = tasks.filter(t => t.total_tokens > 0);
  if (isLoading) return <div className="rounded-xl border border-border bg-card p-5 h-32 bg-muted/20 animate-shimmer" />;
  if (active.length === 0) return null;

  const sorted = [...active].sort((a, b) => b.total_tokens - a.total_tokens);
  const maxTokens = sorted[0].total_tokens;

  const tiles = sorted.map((t, i) => ({
    ...t,
    // Width as % of row: proportional to sqrt of token fraction (so big tasks don't overwhelm)
    pct: Math.max(8, Math.sqrt(t.total_tokens / maxTokens) * 100),
    cost: estimateCost(t.input_tokens, t.output_tokens, t.cache_creation_tokens, t.cache_read_tokens),
    color: seriesColor(i),
  }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Token Allocation by Task</h3>
        <span className="text-[10px] text-muted-foreground">{active.length} tasks · click to filter</span>
      </div>

      <div className="flex flex-wrap gap-1.5">
        {tiles.map(b => {
          const isHovered = hoverId === b.task_id;
          const isSelected = selectedTaskId === b.task_id;
          const label = b.task_title.length > 20 ? b.task_title.slice(0, 19) + '…' : b.task_title;
          return (
            <button
              key={b.task_id}
              onClick={() => onTaskSelect?.(b.task_id)}
              onMouseEnter={() => setHoverId(b.task_id)}
              onMouseLeave={() => setHoverId(null)}
              title={`${b.task_title}\n${fmt(b.total_tokens)} tokens · $${b.cost.toFixed(3)}`}
              style={{
                backgroundColor: b.color,
                opacity: isHovered ? 1 : 0.82,
                outline: isSelected ? '2px solid var(--background)' : undefined,
                outlineOffset: isSelected ? '1px' : undefined,
                flexBasis: `max(120px, ${b.pct}%)`,
                flexGrow: 1,
                flexShrink: 0,
                maxWidth: '100%',
                transition: 'opacity 0.15s',
              }}
              className="rounded-lg p-2.5 text-left cursor-pointer"
            >
              <p className="text-[11px] font-semibold leading-tight truncate" style={{ color: 'var(--background)', textShadow: '0 1px 3px rgba(0,0,0,0.35)' }}>{label}</p>
              <p className="text-[10px] leading-tight mt-0.5" style={{ color: 'var(--background)', opacity: 0.8, textShadow: '0 1px 2px rgba(0,0,0,0.3)' }}>{fmt(b.total_tokens)} · ${b.cost.toFixed(2)}</p>
            </button>
          );
        })}
      </div>
    </div>
  );
}
