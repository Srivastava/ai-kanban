'use client';

import { useState } from 'react';
import { LineChart, Line, BarChart, Bar, Cell, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useTokensByTask, useTaskTimeline } from '@/hooks/use-analytics';
import type { TaskTimelineEvent } from '@/types/analytics';

const SESSION_COLORS = ['#6366f1', '#f97316', '#22c55e', '#ef4444', '#a855f7', '#06b6d4', '#eab308', '#ec4899'];

function sessionColor(idx: number): string {
  return SESSION_COLORS[idx % SESSION_COLORS.length];
}

function formatElapsed(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  const m = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function formatTokens(v: number) {
  return v >= 1000 ? `${(v / 1000).toFixed(0)}K` : String(v);
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

function groupBySessions(events: TaskTimelineEvent[]) {
  const sessionMap = new Map<string, TaskTimelineEvent[]>();
  for (const e of events) {
    if (!sessionMap.has(e.claude_session_id)) sessionMap.set(e.claude_session_id, []);
    sessionMap.get(e.claude_session_id)!.push(e);
  }

  return Array.from(sessionMap.entries()).map(([id, evts], idx) => {
    const t0 = new Date(evts[0].timestamp).getTime();
    const points = evts.map((e) => ({
      elapsed: Math.round((new Date(e.timestamp).getTime() - t0) / 1000),
      cumulative: e.cumulative_total,
      tool: e.tool_name ?? e.event_type,
      input: e.input_tokens,
      output: e.output_tokens,
      total: e.input_tokens + e.output_tokens,
    }));
    return { id, shortId: id.slice(0, 8), color: sessionColor(idx), points };
  });
}

export function SessionTimelineChart() {
  const { data: tasks = [] } = useTokensByTask();
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const { data: events = [], isLoading } = useTaskTimeline(selectedTaskId);

  const sessions = groupBySessions(events);
  const maxElapsed = Math.max(0, ...sessions.flatMap((s) => s.points.map((p) => p.elapsed)));

  // Flat list of all events with session color for the bar chart
  const barData = sessions.flatMap((s) =>
    s.points.map((p) => ({
      ...p,
      elapsedLabel: formatElapsed(p.elapsed),
      color: s.color,
      shortId: s.shortId,
    }))
  );

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center gap-4">
        <h3 className="font-semibold">Session Token Timeline</h3>
        <select
          className="flex-1 max-w-xs rounded-md border border-border bg-background px-3 py-1.5 text-sm"
          value={selectedTaskId ?? ''}
          onChange={(e) => setSelectedTaskId(e.target.value || null)}
        >
          <option value="">Select a task...</option>
          {tasks.map((t) => (
            <option key={t.task_id} value={t.task_id}>
              {t.task_title} — {t.total_tokens.toLocaleString()} tokens
            </option>
          ))}
        </select>
      </div>

      {!selectedTaskId ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">Select a task to view its session timelines</p>
        </div>
      ) : isLoading ? (
        <div className="h-96 animate-pulse bg-muted rounded" />
      ) : sessions.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No timeline data for this task</p>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Session legend */}
          <div className="flex flex-wrap gap-x-4 gap-y-1">
            {sessions.map((s) => (
              <span key={s.id} className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <span className="w-2.5 h-2.5 rounded-full inline-block shrink-0" style={{ background: s.color }} />
                Session {s.shortId}
              </span>
            ))}
          </div>

          {/* Cumulative tokens over elapsed time — one line per claude session */}
          <div>
            <p className="text-xs text-muted-foreground mb-2 font-medium">Cumulative tokens over time (per session)</p>
            <ResponsiveContainer width="100%" height={200}>
              <LineChart margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis
                  type="number"
                  dataKey="elapsed"
                  domain={[0, maxElapsed]}
                  tickFormatter={formatElapsed}
                  tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                  label={{ value: 'Elapsed time', position: 'insideBottom', offset: -15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }}
                />
                <YAxis
                  tickFormatter={formatTokens}
                  tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                  label={{ value: 'Cumulative tokens', angle: -90, position: 'insideLeft', offset: 15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }}
                />
                <Tooltip
                  content={({ active, payload }) => {
                    if (!active || !payload?.length) return null;
                    const d = payload[0].payload;
                    return (
                      <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1">
                        <p className="font-medium" style={{ color: payload[0].color }}>Session {(payload[0] as any).name}</p>
                        <p className="text-muted-foreground">{formatElapsed(d.elapsed)} elapsed</p>
                        <p>Cumulative: {d.cumulative.toLocaleString()}</p>
                      </div>
                    );
                  }}
                  contentStyle={TOOLTIP_STYLE}
                />
                {sessions.map((s) => (
                  <Line
                    key={s.id}
                    name={s.shortId}
                    data={s.points}
                    dataKey="cumulative"
                    stroke={s.color}
                    strokeWidth={2}
                    dot={false}
                    type="monotone"
                    isAnimationActive={false}
                  />
                ))}
              </LineChart>
            </ResponsiveContainer>
          </div>

          {/* Per-event tokens, colored by session */}
          <div>
            <p className="text-xs text-muted-foreground mb-2 font-medium">Tokens per tool call (colored by session)</p>
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={barData} margin={{ top: 5, right: 10, left: 10, bottom: 30 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
                <XAxis
                  dataKey="elapsedLabel"
                  tick={false}
                  label={{ value: 'Tool calls in order', position: 'insideBottom', offset: -15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }}
                />
                <YAxis
                  tickFormatter={formatTokens}
                  tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
                  label={{ value: 'Tokens', angle: -90, position: 'insideLeft', offset: 15, style: { fontSize: 10, fill: 'hsl(var(--muted-foreground))' } }}
                />
                <Tooltip
                  content={({ active, payload }) => {
                    if (!active || !payload?.length) return null;
                    const d = payload[0].payload;
                    return (
                      <div className="rounded-lg border border-border bg-card p-3 text-xs space-y-1">
                        <p className="font-medium" style={{ color: d.color }}>Session {d.shortId}</p>
                        <p className="text-muted-foreground">{d.tool} · {d.elapsedLabel} elapsed</p>
                        <p>{d.input} in / {d.output} out = {d.total.toLocaleString()} total</p>
                      </div>
                    );
                  }}
                  contentStyle={TOOLTIP_STYLE}
                />
                <Bar dataKey="total" radius={[1, 1, 0, 0]}>
                  {barData.map((d, i) => (
                    <Cell key={i} fill={d.color} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}
    </div>
  );
}
