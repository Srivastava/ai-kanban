'use client';

import { useState } from 'react';
import { useTokensByTask } from '@/hooks/use-analytics';
import { estimateCost } from '@/lib/pricing';

// Index-based palette
const PALETTE = ['#6366f1','#8b5cf6','#3b82f6','#06b6d4','#10b981','#f59e0b','#ef4444','#64748b','#ec4899','#84cc16'];

function fmt(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

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
    color: PALETTE[i % PALETTE.length],
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
                outline: isSelected ? '2px solid white' : undefined,
                outlineOffset: isSelected ? '1px' : undefined,
                flexBasis: `max(120px, ${b.pct}%)`,
                flexGrow: 1,
                flexShrink: 0,
                maxWidth: '100%',
                transition: 'opacity 0.15s',
              }}
              className="rounded-lg p-2.5 text-left cursor-pointer"
            >
              <p className="text-white text-[11px] font-semibold leading-tight truncate">{label}</p>
              <p className="text-white/75 text-[10px] leading-tight mt-0.5">{fmt(b.total_tokens)} · ${b.cost.toFixed(2)}</p>
            </button>
          );
        })}
      </div>
    </div>
  );
}
