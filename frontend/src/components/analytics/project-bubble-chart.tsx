'use client';

import { useState } from 'react';
import { useTokensByTask } from '@/hooks/use-analytics';
import { estimateCost } from '@/lib/pricing';

// Index-based palette — no stage data available from the endpoint
const PALETTE = ['#6366f1','#8b5cf6','#3b82f6','#06b6d4','#10b981','#f59e0b','#ef4444','#64748b'];

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
  if (isLoading) return <div className="rounded-xl border border-border bg-card p-5 h-48 animate-pulse bg-muted/20" />;
  if (active.length === 0) return null;

  // Sort by total_tokens desc, compute radii
  const sorted = [...active].sort((a, b) => b.total_tokens - a.total_tokens);
  const maxTokens = sorted[0].total_tokens;
  const MIN_R = 20, MAX_R = 80;

  const bubbles = sorted.map((t, i) => ({
    ...t,
    r: MIN_R + Math.sqrt(t.total_tokens / maxTokens) * (MAX_R - MIN_R),
    cost: estimateCost(t.input_tokens, t.output_tokens, t.cache_creation_tokens, t.cache_read_tokens),
    color: PALETTE[i % PALETTE.length],
  }));

  // Simple row-packing layout
  const SVG_W = 800;
  const PAD = 10;
  let x = PAD, y = PAD, rowMaxH = 0;
  const positioned = bubbles.map(b => {
    const diameter = b.r * 2 + 8;
    if (x + diameter > SVG_W - PAD && x > PAD) {
      x = PAD;
      y += rowMaxH + 8;
      rowMaxH = 0;
    }
    const cx = x + b.r;
    const cy = y + b.r;
    x += diameter + 6;
    rowMaxH = Math.max(rowMaxH, b.r * 2);
    return { ...b, cx, cy };
  });
  const svgH = y + rowMaxH + PAD + 20;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Token Allocation by Task</h3>
      </div>
      <div className="overflow-x-auto">
        <svg width="100%" viewBox={`0 0 ${SVG_W} ${svgH}`} style={{ minWidth: 400 }}>
          {positioned.map(b => {
            const isHovered = hoverId === b.task_id;
            const isSelected = selectedTaskId === b.task_id;
            return (
              <g key={b.task_id}
                className="cursor-pointer"
                onClick={() => onTaskSelect?.(b.task_id)}
                onMouseEnter={() => setHoverId(b.task_id)}
                onMouseLeave={() => setHoverId(null)}>
                <circle
                  cx={b.cx} cy={b.cy} r={b.r}
                  fill={b.color}
                  fillOpacity={isHovered ? 0.9 : 0.7}
                  stroke={isSelected ? '#fff' : 'transparent'}
                  strokeWidth={isSelected ? 2.5 : 0}
                  style={{ transition: 'fill-opacity 0.15s' }}
                />
                {b.r > 40 && (
                  <text x={b.cx} y={b.cy - 6} textAnchor="middle" fontSize={Math.min(11, b.r / 4)}
                    fill="white" fontWeight={500}
                    style={{ pointerEvents: 'none' }}>
                    {b.task_title.length > 14 ? b.task_title.slice(0, 13) + '…' : b.task_title}
                  </text>
                )}
                {b.r > 40 && (
                  <text x={b.cx} y={b.cy + 9} textAnchor="middle" fontSize={Math.min(10, b.r / 5)}
                    fill="rgba(255,255,255,0.8)"
                    style={{ pointerEvents: 'none' }}>
                    {fmt(b.total_tokens)}
                  </text>
                )}
                {isHovered && (
                  <title>{`${b.task_title}\n${fmt(b.total_tokens)} tokens · $${b.cost.toFixed(3)}`}</title>
                )}
              </g>
            );
          })}
        </svg>
      </div>
      <p className="text-[10px] text-muted-foreground">
        Bubble size = total tokens · click to filter · {active.length} tasks
      </p>
    </div>
  );
}
