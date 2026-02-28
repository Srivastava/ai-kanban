# Analytics Tab Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the `/analytics` page with beautiful Recharts graphs showing token usage across all dimensions (time, task, session, tool, language, efficiency, session timeline).

**Architecture:** New Next.js page at `src/app/analytics/page.tsx`. API calls via `apiClient`. Data types defined in `src/types/analytics.ts`. Charts built with Recharts. Sidebar updated with Analytics link.

**Tech Stack:** Next.js 16 App Router, Recharts, @tanstack/react-query, Tailwind CSS

---

## Context

Backend analytics endpoints (from plan4) available at:
- `GET /api/analytics/overview`
- `GET /api/analytics/tokens/daily?days=30`
- `GET /api/analytics/tokens/weekly?weeks=12`
- `GET /api/analytics/tokens/monthly?months=6`
- `GET /api/analytics/tokens/by-task`
- `GET /api/analytics/tokens/by-session`
- `GET /api/analytics/tokens/by-tool`
- `GET /api/analytics/tokens/by-language`
- `GET /api/analytics/tokens/efficiency`
- `GET /api/analytics/sessions/:id/timeline`

Color palette (dark theme):
- Input tokens: `#6366f1` (indigo-500)
- Output tokens: `#a855f7` (purple-500)
- Background: use Tailwind `bg-card` / `border-border`

---

## Task 1: Install Recharts

**Step 1: Install**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm install recharts
```

**Step 2: Verify**

```bash
npm ls recharts 2>&1 | head -3
```

Expected: `recharts@2.x.x` listed.

---

## Task 2: Analytics Types

**Files:**
- Create: `frontend/src/types/analytics.ts`

**Step 1: Create types matching the backend responses**

Create `frontend/src/types/analytics.ts`:

```typescript
export interface AnalyticsOverview {
  total_input_tokens: number;
  total_output_tokens: number;
  total_sessions: number;
  total_tasks_with_sessions: number;
  estimated_cost_usd: number;
  active_sessions_today: number;
}

export interface DailyTokens {
  date: string;
  input_tokens: number;
  output_tokens: number;
}

export interface WeeklyTokens {
  week_start: string;
  input_tokens: number;
  output_tokens: number;
}

export interface MonthlyTokens {
  month: string;
  input_tokens: number;
  output_tokens: number;
}

export interface TaskTokens {
  task_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
}

export interface SessionTokens {
  session_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  started_at: string | null;
}

export interface ToolTokens {
  tool_name: string;
  input_tokens: number;
  output_tokens: number;
  call_count: number;
}

export interface LanguageTokens {
  file_ext: string;
  input_tokens: number;
  output_tokens: number;
  call_count: number;
}

export interface EfficiencyRow {
  task_id: string;
  task_title: string;
  total_tokens: number;
  lines_written: number;
  project_loc: number;
  tokens_per_line: number | null;
  tokens_per_loc: number | null;
}

export interface SessionTimelineEvent {
  sequence_no: number;
  event_type: string;
  tool_name: string | null;
  input_tokens: number;
  output_tokens: number;
  cumulative_total: number;
  timestamp: string;
}
```

---

## Task 3: Analytics API Hooks

**Files:**
- Create: `frontend/src/hooks/use-analytics.ts`

**Step 1: Create hooks**

Create `frontend/src/hooks/use-analytics.ts`:

```typescript
'use client';

import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type {
  AnalyticsOverview,
  DailyTokens,
  EfficiencyRow,
  LanguageTokens,
  MonthlyTokens,
  SessionTimelineEvent,
  SessionTokens,
  TaskTokens,
  ToolTokens,
  WeeklyTokens,
} from '@/types/analytics';

export function useAnalyticsOverview() {
  return useQuery({
    queryKey: ['analytics', 'overview'],
    queryFn: () => apiClient<AnalyticsOverview>('/api/analytics/overview'),
    refetchInterval: 30_000,
  });
}

export function useDailyTokens(days = 30) {
  return useQuery({
    queryKey: ['analytics', 'daily', days],
    queryFn: () => apiClient<DailyTokens[]>(`/api/analytics/tokens/daily?days=${days}`),
  });
}

export function useWeeklyTokens(weeks = 12) {
  return useQuery({
    queryKey: ['analytics', 'weekly', weeks],
    queryFn: () => apiClient<WeeklyTokens[]>(`/api/analytics/tokens/weekly?weeks=${weeks}`),
  });
}

export function useMonthlyTokens(months = 6) {
  return useQuery({
    queryKey: ['analytics', 'monthly', months],
    queryFn: () => apiClient<MonthlyTokens[]>(`/api/analytics/tokens/monthly?months=${months}`),
  });
}

export function useTokensByTask() {
  return useQuery({
    queryKey: ['analytics', 'by-task'],
    queryFn: () => apiClient<TaskTokens[]>('/api/analytics/tokens/by-task'),
  });
}

export function useTokensBySession() {
  return useQuery({
    queryKey: ['analytics', 'by-session'],
    queryFn: () => apiClient<SessionTokens[]>('/api/analytics/tokens/by-session'),
  });
}

export function useTokensByTool() {
  return useQuery({
    queryKey: ['analytics', 'by-tool'],
    queryFn: () => apiClient<ToolTokens[]>('/api/analytics/tokens/by-tool'),
  });
}

export function useTokensByLanguage() {
  return useQuery({
    queryKey: ['analytics', 'by-language'],
    queryFn: () => apiClient<LanguageTokens[]>('/api/analytics/tokens/by-language'),
  });
}

export function useTokenEfficiency() {
  return useQuery({
    queryKey: ['analytics', 'efficiency'],
    queryFn: () => apiClient<EfficiencyRow[]>('/api/analytics/tokens/efficiency'),
  });
}

export function useSessionTimeline(sessionId: string | null) {
  return useQuery({
    queryKey: ['analytics', 'timeline', sessionId],
    queryFn: () =>
      apiClient<SessionTimelineEvent[]>(`/api/analytics/sessions/${sessionId}/timeline`),
    enabled: !!sessionId,
  });
}
```

---

## Task 4: Summary Cards Component

**Files:**
- Create: `frontend/src/components/analytics/overview-cards.tsx`

**Step 1: Create**

Create `frontend/src/components/analytics/overview-cards.tsx`:

```tsx
'use client';

import { useAnalyticsOverview } from '@/hooks/use-analytics';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function OverviewCards() {
  const { data, isLoading } = useAnalyticsOverview();

  const cards = [
    {
      label: 'Total Tokens',
      value: data
        ? formatTokens(data.total_input_tokens + data.total_output_tokens)
        : '—',
      sub: data
        ? `${formatTokens(data.total_input_tokens)} in / ${formatTokens(data.total_output_tokens)} out`
        : '',
    },
    {
      label: 'Estimated Cost',
      value: data ? `$${data.estimated_cost_usd.toFixed(4)}` : '—',
      sub: 'Claude Sonnet pricing',
    },
    {
      label: 'Total Sessions',
      value: data ? data.total_sessions.toString() : '—',
      sub: `${data?.active_sessions_today ?? 0} today`,
    },
    {
      label: 'Tasks with AI',
      value: data ? data.total_tasks_with_sessions.toString() : '—',
      sub: 'tasks with ≥1 session',
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {cards.map((card) => (
        <div
          key={card.label}
          className="rounded-xl border border-border bg-card p-5 space-y-1"
        >
          <p className="text-xs text-muted-foreground uppercase tracking-wider">
            {card.label}
          </p>
          <p className="text-2xl font-bold">
            {isLoading ? (
              <span className="animate-pulse bg-muted rounded w-16 h-7 inline-block" />
            ) : (
              card.value
            )}
          </p>
          <p className="text-xs text-muted-foreground">{card.sub}</p>
        </div>
      ))}
    </div>
  );
}
```

---

## Task 5: Token Time-Series Chart

**Files:**
- Create: `frontend/src/components/analytics/token-time-chart.tsx`

**Step 1: Create**

Create `frontend/src/components/analytics/token-time-chart.tsx`:

```tsx
'use client';

import { useState } from 'react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { useDailyTokens, useWeeklyTokens, useMonthlyTokens } from '@/hooks/use-analytics';

type Period = 'daily' | 'weekly' | 'monthly';

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function TokenTimeChart() {
  const [period, setPeriod] = useState<Period>('daily');

  const daily = useDailyTokens(30);
  const weekly = useWeeklyTokens(12);
  const monthly = useMonthlyTokens(6);

  const dataMap = {
    daily: (daily.data ?? []).map((d) => ({
      label: d.date.slice(5),       // "MM-DD"
      input: d.input_tokens,
      output: d.output_tokens,
    })),
    weekly: (weekly.data ?? []).map((d) => ({
      label: d.week_start.slice(5),
      input: d.input_tokens,
      output: d.output_tokens,
    })),
    monthly: (monthly.data ?? []).map((d) => ({
      label: d.month,
      input: d.input_tokens,
      output: d.output_tokens,
    })),
  };

  const data = dataMap[period];
  const isLoading = { daily, weekly, monthly }[period].isLoading;

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Token Usage Over Time</h3>
        <div className="flex gap-1">
          {(['daily', 'weekly', 'monthly'] as Period[]).map((p) => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                period === p
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-muted'
              }`}
            >
              {p.charAt(0).toUpperCase() + p.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground text-sm">Loading...</div>
        </div>
      ) : data.length === 0 ? (
        <div className="h-64 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No token data yet. Run a Claude session to see usage.</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={256}>
          <AreaChart data={data} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
            <defs>
              <linearGradient id="inputGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="outputGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#a855f7" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#a855f7" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis dataKey="label" tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} />
            <YAxis tickFormatter={formatTokens} tick={{ fontSize: 11, fill: 'hsl(var(--muted-foreground))' }} />
            <Tooltip
              formatter={(value: number, name: string) => [
                formatTokens(value),
                name === 'input' ? 'Input tokens' : 'Output tokens',
              ]}
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: '12px',
              }}
            />
            <Legend
              formatter={(value) => (value === 'input' ? 'Input' : 'Output')}
              iconType="circle"
            />
            <Area type="monotone" dataKey="input" stroke="#6366f1" fill="url(#inputGrad)" strokeWidth={2} dot={false} />
            <Area type="monotone" dataKey="output" stroke="#a855f7" fill="url(#outputGrad)" strokeWidth={2} dot={false} />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
```

---

## Task 6: Tool Breakdown + Language Chart

**Files:**
- Create: `frontend/src/components/analytics/tool-breakdown-chart.tsx`
- Create: `frontend/src/components/analytics/language-chart.tsx`

**Step 1: Tool breakdown (donut)**

Create `frontend/src/components/analytics/tool-breakdown-chart.tsx`:

```tsx
'use client';

import { PieChart, Pie, Cell, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { useTokensByTool } from '@/hooks/use-analytics';

const COLORS = ['#6366f1', '#a855f7', '#ec4899', '#f97316', '#eab308', '#22c55e', '#06b6d4'];

export function ToolBreakdownChart() {
  const { data = [], isLoading } = useTokensByTool();

  const chartData = data.map((d) => ({
    name: d.tool_name,
    value: d.input_tokens + d.output_tokens,
    calls: d.call_count,
  }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Tokens per Tool Call</h3>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No tool data yet</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={192}>
          <PieChart>
            <Pie
              data={chartData}
              cx="50%"
              cy="50%"
              innerRadius={50}
              outerRadius={80}
              paddingAngle={3}
              dataKey="value"
            >
              {chartData.map((_, i) => (
                <Cell key={i} fill={COLORS[i % COLORS.length]} />
              ))}
            </Pie>
            <Tooltip
              formatter={(value: number, name: string) => [
                value.toLocaleString() + ' tokens',
                name,
              ]}
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: '12px',
              }}
            />
            <Legend iconType="circle" iconSize={8} />
          </PieChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
```

**Step 2: Language chart**

Create `frontend/src/components/analytics/language-chart.tsx`:

```tsx
'use client';

import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useTokensByLanguage } from '@/hooks/use-analytics';

export function LanguageChart() {
  const { data = [], isLoading } = useTokensByLanguage();

  const chartData = data.slice(0, 10).map((d) => ({
    ext: d.file_ext,
    tokens: d.input_tokens + d.output_tokens,
    calls: d.call_count,
  }));

  return (
    <div className="rounded-xl border border-border bg-card p-5 space-y-4">
      <h3 className="font-semibold">Tokens per Language</h3>
      {isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No language data yet</p>
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={192}>
          <BarChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
            <XAxis dataKey="ext" tick={{ fontSize: 11 }} />
            <YAxis tick={{ fontSize: 11 }} />
            <Tooltip
              contentStyle={{
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '8px',
                fontSize: '12px',
              }}
            />
            <Bar dataKey="tokens" fill="#6366f1" radius={[4, 4, 0, 0]} name="Tokens" />
          </BarChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
```

---

## Task 7: Session Timeline Chart

**Files:**
- Create: `frontend/src/components/analytics/session-timeline-chart.tsx`

**Step 1: Create**

Create `frontend/src/components/analytics/session-timeline-chart.tsx`:

```tsx
'use client';

import { useState } from 'react';
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { useTokensBySession, useSessionTimeline } from '@/hooks/use-analytics';

export function SessionTimelineChart() {
  const { data: sessions = [] } = useTokensBySession();
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const { data: timeline = [], isLoading } = useSessionTimeline(selectedSessionId);

  const chartData = timeline.map((e) => ({
    seq: e.sequence_no,
    cumulative: e.cumulative_total,
    tool: e.tool_name ?? e.event_type,
    input: e.input_tokens,
    output: e.output_tokens,
  }));

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
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">Select a session to view its token timeline</p>
        </div>
      ) : isLoading ? (
        <div className="h-48 animate-pulse bg-muted rounded" />
      ) : chartData.length === 0 ? (
        <div className="h-48 flex items-center justify-center">
          <p className="text-muted-foreground text-sm">No timeline data for this session</p>
        </div>
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
            <XAxis dataKey="seq" label={{ value: 'Event #', position: 'insideBottom', offset: -2, fontSize: 11 }} />
            <YAxis tickFormatter={(v) => v >= 1000 ? `${(v/1000).toFixed(0)}K` : v} />
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
            <Area type="monotone" dataKey="cumulative" stroke="#6366f1" fill="url(#cumGrad)" strokeWidth={2} dot={false} name="Cumulative tokens" />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
```

---

## Task 8: Analytics Page

**Files:**
- Create: `frontend/src/app/analytics/page.tsx`
- Modify: `frontend/src/components/layout/sidebar.tsx`

**Step 1: Create the page**

Create `frontend/src/app/analytics/page.tsx`:

```tsx
import { Sidebar } from '@/components/layout/sidebar';
import { OverviewCards } from '@/components/analytics/overview-cards';
import { TokenTimeChart } from '@/components/analytics/token-time-chart';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';
import { SessionTimelineChart } from '@/components/analytics/session-timeline-chart';

export default function AnalyticsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <div className="border-b border-border px-6 py-4">
          <h1 className="text-xl font-semibold">Analytics</h1>
          <p className="text-sm text-muted-foreground">Claude token usage and efficiency</p>
        </div>
        <main className="flex-1 p-6 space-y-6">
          {/* Row 1: Summary cards */}
          <OverviewCards />

          {/* Row 2: Time-series */}
          <TokenTimeChart />

          {/* Row 3: Breakdowns */}
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <ToolBreakdownChart />
            <LanguageChart />
          </div>

          {/* Row 4: Session timeline */}
          <SessionTimelineChart />
        </main>
      </div>
    </div>
  );
}
```

**Step 2: Add Analytics link to sidebar**

Open `frontend/src/components/layout/sidebar.tsx`. Add after the Kanban Board link:

```tsx
<Link
  href="/analytics"
  className={cn(
    'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
    pathname === '/analytics'
      ? 'bg-sidebar-accent text-sidebar-accent-foreground'
      : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
  )}
>
  Analytics
</Link>
```

Also add `'/analytics'` to the analytics `isActive` check or use `pathname === '/analytics'` directly as above.

**Step 3: Verify TypeScript and build**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit 2>&1 | head -30
npm run build 2>&1 | tail -10
```

Expected: no type errors, build succeeds.

**Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/src/types/analytics.ts \
        frontend/src/hooks/use-analytics.ts \
        frontend/src/components/analytics/ \
        frontend/src/app/analytics/page.tsx \
        frontend/src/components/layout/sidebar.tsx
git commit -m "feat(frontend): add Analytics tab with Recharts graphs

- Overview cards: total tokens, cost, sessions, tasks with AI
- Token time-series area chart with Daily/Weekly/Monthly toggle
- Tool breakdown donut chart (per tool call token usage)
- Language bar chart (tokens by file extension)
- Session timeline area chart (cumulative tokens per JSONL event)
- Sidebar updated with Analytics nav link"
```
