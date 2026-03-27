'use client';

import { useState, useMemo, useRef, useEffect } from 'react';
import { useDailyHeatmap } from '@/hooks/use-analytics';
import { formatTokens } from '@/lib/format';

// Heatmap intensity levels — CSS tokens defined in globals.css.
// light mode: dark-on-white ramp; dark mode: light-on-dark ramp.
// Index 0 = no activity; indices 1–4 = increasing intensity.
const HEATMAP_COLORS = [
  'var(--muted)',
  'var(--heatmap-1)',
  'var(--heatmap-2)',
  'var(--heatmap-3)',
  'var(--heatmap-4)',
] as const;

function tokenColor(tokens: number, max: number): string {
  if (tokens === 0 || max === 0) return HEATMAP_COLORS[0];
  const idx = Math.ceil((tokens / max) * 4);
  return HEATMAP_COLORS[Math.min(idx, 4)];
}

interface Props { taskId?: string | null }

export function ActivityHeatmap({ taskId }: Props) {
  const { data = [], isLoading } = useDailyHeatmap(365, taskId);
  const [tooltip, setTooltip] = useState<{ date: string; tokens: number; x: number; y: number } | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const { byDate, max, weeks, todayStr } = useMemo(() => {
    const byDate = new Map(data.map(d => [d.date, d.tokens]));
    const max = Math.max(...data.map(d => d.tokens), 1);

    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const todayStr = today.toISOString().slice(0, 10);
    const start = new Date(today);
    start.setDate(start.getDate() - 364 - start.getDay());

    const weeks: Array<Array<{ date: string; tokens: number }>> = [];
    const cursor = new Date(start);
    for (let w = 0; w < 53; w++) {
      const week: Array<{ date: string; tokens: number }> = [];
      for (let d = 0; d < 7; d++) {
        const dateStr = cursor.toISOString().slice(0, 10);
        week.push({ date: dateStr, tokens: byDate.get(dateStr) ?? 0 });
        cursor.setDate(cursor.getDate() + 1);
      }
      weeks.push(week);
    }

    return { byDate, max, weeks, todayStr };
  }, [data]);

  // Scroll so today (rightmost column) is visible on mount
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollLeft = scrollRef.current.scrollWidth;
    }
  }, [isLoading]);

  const CELL = 11;
  const GAP = 2;
  const stride = CELL + GAP;
  const svgW = weeks.length * stride;
  const svgH = 7 * stride;
  const MONTHS = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  const DAYS = ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-3">
      <h3 className="font-semibold text-sm">Activity Heatmap</h3>
      {isLoading ? (
        <div className="h-24 bg-muted rounded animate-shimmer" />
      ) : (
        <div className="overflow-x-auto relative" ref={scrollRef}>
          <div className="flex gap-3 items-start min-w-max">
            {/* Day labels */}
            <svg width={28} height={svgH + 18} className="shrink-0 mt-5" aria-hidden="true">
              {[1, 3, 5].map(d => (
                <text key={d} x={24} y={d * stride + CELL - 1} fontSize={9}
                  fill="var(--muted-foreground)" textAnchor="end">{DAYS[d]}</text>
              ))}
            </svg>
            {/* Main grid */}
            <div className="relative">
              {/* Month labels */}
              <svg width={svgW} height={14} className="block mb-1" aria-hidden="true">
                {weeks.map((week, wi) => {
                  const month = new Date(week[0].date).getUTCMonth();
                  const prevMonth = wi > 0 ? new Date(weeks[wi - 1][0].date).getUTCMonth() : -1;
                  if (month !== prevMonth) {
                    return <text key={wi} x={wi * stride} y={11} fontSize={9}
                      fill="var(--muted-foreground)">{MONTHS[month]}</text>;
                  }
                  return null;
                })}
              </svg>
              <svg
                width={svgW}
                height={svgH}
                role="img"
                aria-label="Daily activity heatmap — token usage over the past year"
                onMouseLeave={() => setTooltip(null)}
              >
                {weeks.map((week, wi) =>
                  week.map((cell, di) => {
                    if (cell.date > todayStr) return null;
                    return (
                      <rect
                        key={`${wi}-${di}`}
                        x={wi * stride} y={di * stride}
                        width={CELL} height={CELL}
                        rx={2}
                        fill={tokenColor(cell.tokens, max)}
                        onMouseEnter={(e) => {
                          const rect = (e.target as SVGRectElement).getBoundingClientRect();
                          setTooltip({ date: cell.date, tokens: cell.tokens, x: rect.left, y: rect.top });
                        }}
                      />
                    );
                  })
                )}
              </svg>
              {tooltip && (
                <div className="fixed z-50 pointer-events-none bg-card border border-border rounded px-2 py-1 text-xs shadow"
                  style={{ left: tooltip.x + 14, top: tooltip.y - 30 }}>
                  <span className="font-medium">{tooltip.date}</span>
                  {' — '}
                  {tooltip.tokens > 0 ? `${formatTokens(tooltip.tokens)} tokens` : 'no activity'}
                </div>
              )}
            </div>
            {/* Legend */}
            <div className="flex flex-col gap-1 mt-5 ml-2 shrink-0" aria-hidden="true">
              <span className="text-[9px] text-muted-foreground">Less</span>
              {HEATMAP_COLORS.map((c, i) => (
                <div key={i} className="w-2.5 h-2.5 rounded-[2px]" style={{ background: c }} />
              ))}
              <span className="text-[9px] text-muted-foreground">More</span>
            </div>
          </div>
        </div>
      )}
      <p className="text-[10px] text-muted-foreground">
        Total tokens per day (UTC) · last 365 days · {data.length} active days
      </p>
    </div>
  );
}
