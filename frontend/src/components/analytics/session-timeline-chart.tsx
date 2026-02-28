'use client';

import { useState } from 'react';
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useTokensBySession, useSessionTimeline } from '@/hooks/use-analytics';

export function SessionTimelineChart() {
  const { data: sessions = [] } = useTokensBySession();
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const { data: timeline = [], isLoading } = useSessionTimeline(selectedSessionId);

  const chartData = timeline.map((e) => ({ seq: e.sequence_no, cumulative: e.cumulative_total, tool: e.tool_name ?? e.event_type, input: e.input_tokens, output: e.output_tokens }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center gap-4">
        <h3 className="font-semibold">Session Token Timeline</h3>
        <select
          className="flex-1 max-w-xs rounded-md border border-border bg-background px-3 py-1.5 text-sm"
          value={selectedSessionId ?? ''}
          onChange={(e) => setSelectedSessionId(e.target.value || null)}
        >
          <option value="">Select a session...</option>
          {sessions.map((s) => (
            <option key={s.session_id} value={s.session_id}>
              {s.task_title} — {s.total_tokens.toLocaleString()} tokens
            </option>
          ))}
        </select>
      </div>

      {!selectedSessionId ? (
        <div className="h-48 flex items-center justify-center"><p className="text-muted-foreground text-sm">Select a session to view its token timeline</p></div>
      ) : isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center"><p className="text-muted-foreground text-sm">No timeline data for this session</p></div>
      ) : (
        <ResponsiveContainer width="100%" height={240}>
          <AreaChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
            <defs>
              <linearGradient id="cumGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.4} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis dataKey="seq" />
            <YAxis tickFormatter={(v) => (v >= 1000 ? `${(v / 1000).toFixed(0)}K` : v)} />
            <Tooltip
              content={({ active, payload }) => {
                if (!active || !payload?.length) return null;
                const d = payload[0].payload;
                return (
                  <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1">
                    <p className="font-medium">{d.tool}</p>
                    <p>Cumulative: {d.cumulative.toLocaleString()}</p>
                    <p>Input: {d.input} / Output: {d.output}</p>
                  </div>
                );
              }}
            />
            <Area type="monotone" dataKey="cumulative" stroke="#6366f1" fill="url(#cumGrad)" strokeWidth={2} dot={false} />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
