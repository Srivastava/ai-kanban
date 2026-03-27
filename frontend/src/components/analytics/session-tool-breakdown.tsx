'use client';

import { useEffect } from 'react';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { useSessionTools } from '@/hooks/use-analytics';
import { X } from 'lucide-react';

function fmt(n: number) {
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

interface Props {
  sessionId: string;
  sessionTotalTokens: number;
  onClose: () => void;
}

export function SessionToolBreakdown({ sessionId, sessionTotalTokens, onClose }: Props) {
  const { data = [], isLoading } = useSessionTools(sessionId);

  // Data integrity check
  useEffect(() => {
    if (data.length === 0) return;
    const sum = data.reduce((acc, t) => acc + t.input_tokens + t.output_tokens, 0);
    if (Math.abs(sum - sessionTotalTokens) > 10) {
      console.warn(`[SessionToolBreakdown] token mismatch: breakdown=${sum}, session=${sessionTotalTokens}`);
    }
  }, [data, sessionTotalTokens]);

  const chartData = data.map(t => ({
    name: t.tool_name,
    total: t.input_tokens + t.output_tokens,
    calls: t.call_count,
  }));

  const COLORS = ['#6366f1','#8b5cf6','#3b82f6','#06b6d4','#10b981','#f59e0b','#ef4444','#64748b'];

  return (
    <div className="mt-2 rounded-lg border border-border bg-muted/20 p-4 space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          Tool breakdown · session {sessionId.slice(0, 8)}
        </p>
        <button onClick={onClose} className="p-1 rounded hover:bg-muted">
          <X className="h-3.5 w-3.5 text-muted-foreground" />
        </button>
      </div>

      {isLoading ? (
        <div className="h-24 bg-muted rounded animate-shimmer" />
      ) : data.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">No tool call data for this session</p>
      ) : (
        <>
          <ResponsiveContainer width="100%" height={Math.max(80, data.length * 28)}>
            <BarChart data={chartData} layout="vertical" margin={{ top: 0, right: 60, left: 10, bottom: 0 }}>
              <XAxis type="number" tickFormatter={fmt} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <YAxis type="category" dataKey="name" tick={{ fontSize: 11, fill: 'hsl(var(--foreground))' }} width={80} />
              <Tooltip
                formatter={(v, _name, props) => [
                  `${fmt(Number(v))} tokens · ${props.payload.calls} calls`,
                  'Total',
                ]}
                contentStyle={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border))', borderRadius: 6, fontSize: 11, color: 'hsl(var(--card-foreground))' }}
              />
              <Bar dataKey="total" radius={[0, 3, 3, 0]}>
                {chartData.map((_, i) => <Cell key={i} fill={COLORS[i % COLORS.length]} />)}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
          <div className="flex gap-4 text-xs text-muted-foreground">
            {data.map((t, i) => (
              <span key={t.tool_name} className="flex items-center gap-1">
                <span className="w-2 h-2 rounded-sm" style={{ background: COLORS[i % COLORS.length] }} />
                {t.tool_name}: {t.call_count}×
              </span>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
