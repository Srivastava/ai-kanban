'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend } from 'recharts';
import { useDevActivity } from '@/hooks/use-analytics';

function formatTime(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  return `${(secs / 3600).toFixed(1)}h`;
}

function truncate(s: string, n = 14): string {
  return s.length > n ? s.slice(0, n) + '\u2026' : s;
}

const TOOLTIP_STYLE = {
  background: 'hsl(var(--card))',
  border: '1px solid hsl(var(--border))',
  borderRadius: '8px',
  fontSize: '12px',
};

export function DevActivityCharts() {
  const { data = [], isLoading } = useDevActivity();

  const chartData = data.map((row) => ({
    label: truncate(row.task_title),
    fullTitle: row.task_title,
    added: Math.round(row.lines_added),
    deleted: Math.round(row.lines_deleted),
    active_time: row.active_time_secs,
    commits: Math.round(row.commits),
    prs: Math.round(row.pull_requests),
  }));

  const empty = (
    <div className="h-48 flex items-center justify-center">
      <p className="text-muted-foreground text-sm">No OTel data yet — run a task to see activity</p>
    </div>
  );

  const skeleton = <div className="h-48 animate-pulse bg-muted rounded" />;

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
      {/* Lines of Code */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Lines of Code</h3>
        {isLoading ? skeleton : chartData.length === 0 ? empty : (
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <YAxis tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
                formatter={(value, name) => [value, name === 'added' ? 'Added' : 'Deleted']}
                contentStyle={TOOLTIP_STYLE}
              />
              <Legend formatter={(v) => (v === 'added' ? 'Added' : 'Deleted')} iconType="circle" />
              <Bar dataKey="added" fill="#22c55e" name="added" radius={[3, 3, 0, 0]} />
              <Bar dataKey="deleted" fill="#ef4444" name="deleted" radius={[3, 3, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Active Coding Time */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Active Coding Time</h3>
        {isLoading ? skeleton : chartData.length === 0 ? empty : (
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <YAxis tickFormatter={formatTime} tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
                formatter={(value) => [formatTime(Number(value)), 'Active Time']}
                contentStyle={TOOLTIP_STYLE}
              />
              <Bar dataKey="active_time" fill="#6366f1" name="active_time" radius={[3, 3, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Commits & PRs */}
      <div className="rounded-xl border border-border bg-card p-5 space-y-3">
        <h3 className="font-semibold text-sm">Commits & Pull Requests</h3>
        {isLoading ? skeleton : chartData.length === 0 ? empty : (
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" vertical={false} />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <YAxis tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }} />
              <Tooltip
                labelFormatter={(_, payload) => payload?.[0]?.payload?.fullTitle ?? ''}
                formatter={(value, name) => [value, name === 'commits' ? 'Commits' : 'Pull Requests']}
                contentStyle={TOOLTIP_STYLE}
              />
              <Legend formatter={(v) => (v === 'commits' ? 'Commits' : 'PRs')} iconType="circle" />
              <Bar dataKey="commits" fill="#6366f1" name="commits" radius={[3, 3, 0, 0]} />
              <Bar dataKey="prs" fill="#a855f7" name="prs" radius={[3, 3, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>
    </div>
  );
}
