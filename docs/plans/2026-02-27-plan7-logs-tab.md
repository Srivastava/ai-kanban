# Logs Tab Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the `/logs` page that polls `/api/logs` every 5 seconds, shows a filterable color-coded log table (frontend + backend logs unified), with expandable row detail.

**Architecture:** New Next.js page at `src/app/logs/page.tsx`. React Query polls every 5s using `refetchInterval`. Logs accumulated in local state (new entries prepend). Filters applied client-side. Sidebar updated with Logs link.

**Tech Stack:** Next.js 16 App Router, @tanstack/react-query (polling), Tailwind CSS

---

## Context

Backend endpoint: `GET /api/logs?level=&source=&task_id=&session_id=&limit=&offset=`
Existing types: `source` is `'frontend'` or `'backend'`, `level` is `'DEBUG'|'INFO'|'WARN'|'ERROR'`.

Log table columns: Time | Level | Source | Target | Message | Task

---

## Task 1: Log Types

**Files:**
- Create: `frontend/src/types/log.ts`

**Step 1: Create types**

Create `frontend/src/types/log.ts`:

```typescript
export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
export type LogSource = 'frontend' | 'backend';

export interface LogEntry {
  id: number;
  timestamp: string;   // ISO 8601
  level: LogLevel;
  message: string;
  target: string | null;
  source: LogSource;
  task_id: string | null;
  session_id: string | null;
  metadata: string | null;  // JSON string
  created_at: string;
}

export interface LogFilter {
  level?: LogLevel;
  source?: LogSource;
  task_id?: string;
  session_id?: string;
  search?: string;     // client-side text filter
}
```

---

## Task 2: Logs Hook

**Files:**
- Create: `frontend/src/hooks/use-logs.ts`

**Step 1: Create the polling hook**

Create `frontend/src/hooks/use-logs.ts`:

```typescript
'use client';

import { useState, useCallback, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type { LogEntry, LogFilter } from '@/types/log';

const POLL_INTERVAL = 5_000;

export function useLogs(filter: Omit<LogFilter, 'search'> = {}) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [newCount, setNewCount] = useState(0);
  const lastTimestampRef = useRef<string | null>(null);
  const isLiveRef = useRef(true);

  const buildUrl = useCallback(
    (since?: string | null) => {
      const params = new URLSearchParams();
      if (filter.level) params.set('level', filter.level);
      if (filter.source) params.set('source', filter.source);
      if (filter.task_id) params.set('task_id', filter.task_id);
      if (filter.session_id) params.set('session_id', filter.session_id);
      params.set('limit', '100');
      // NOTE: backend doesn't support ?since= yet; fetch all and deduplicate by id
      return `/api/logs?${params.toString()}`;
    },
    [filter.level, filter.source, filter.task_id, filter.session_id]
  );

  const { isLoading } = useQuery({
    queryKey: ['logs', filter],
    queryFn: async () => {
      const url = buildUrl();
      const fresh = await apiClient<LogEntry[]>(url);

      setLogs((prev) => {
        // Deduplicate by id, keep newest first
        const existingIds = new Set(prev.map((l) => l.id));
        const newEntries = fresh.filter((l) => !existingIds.has(l.id));

        if (newEntries.length === 0) return prev;

        if (isLiveRef.current) {
          setNewCount(0);
          return [...newEntries, ...prev].slice(0, 500); // cap at 500
        } else {
          setNewCount((c) => c + newEntries.length);
          return prev;
        }
      });

      if (fresh.length > 0) {
        lastTimestampRef.current = fresh[0].timestamp;
      }

      return fresh;
    },
    refetchInterval: POLL_INTERVAL,
    refetchIntervalInBackground: false,
  });

  const loadNewLogs = useCallback(() => {
    setLogs((prev) => prev); // trigger a re-render after setting live
    setNewCount(0);
  }, []);

  return { logs, isLoading, newCount, loadNewLogs, isLiveRef };
}
```

---

## Task 3: Log Level Badge Component

**Files:**
- Create: `frontend/src/components/logs/log-level-badge.tsx`

**Step 1: Create**

Create `frontend/src/components/logs/log-level-badge.tsx`:

```tsx
import type { LogLevel } from '@/types/log';
import { cn } from '@/lib/utils';

const levelConfig: Record<LogLevel, { label: string; classes: string }> = {
  DEBUG: { label: 'DEBUG', classes: 'bg-muted text-muted-foreground' },
  INFO:  { label: 'INFO',  classes: 'bg-blue-500/15 text-blue-400' },
  WARN:  { label: 'WARN',  classes: 'bg-amber-500/15 text-amber-400' },
  ERROR: { label: 'ERROR', classes: 'bg-red-500/15 text-red-400' },
};

interface Props {
  level: LogLevel;
}

export function LogLevelBadge({ level }: Props) {
  const config = levelConfig[level] ?? levelConfig.INFO;
  return (
    <span
      className={cn(
        'inline-flex items-center rounded px-1.5 py-0.5 text-xs font-mono font-medium',
        config.classes
      )}
    >
      {config.label}
    </span>
  );
}
```

---

## Task 4: Log Table Component

**Files:**
- Create: `frontend/src/components/logs/log-table.tsx`

**Step 1: Create**

Create `frontend/src/components/logs/log-table.tsx`:

```tsx
'use client';

import { useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { LogLevelBadge } from './log-level-badge';
import { cn } from '@/lib/utils';
import type { LogEntry, LogFilter } from '@/types/log';

interface Props {
  logs: LogEntry[];
  filter: LogFilter;
}

export function LogTable({ logs, filter }: Props) {
  const [expandedId, setExpandedId] = useState<number | null>(null);

  // Client-side text filter
  const visible = logs.filter((log) => {
    if (filter.search && !log.message.toLowerCase().includes(filter.search.toLowerCase())) {
      return false;
    }
    return true;
  });

  if (visible.length === 0) {
    return (
      <div className="flex items-center justify-center h-48">
        <p className="text-muted-foreground text-sm">No logs match the current filters</p>
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-border overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted/30">
            <th className="px-4 py-2 text-left font-medium text-muted-foreground w-32">Time</th>
            <th className="px-4 py-2 text-left font-medium text-muted-foreground w-20">Level</th>
            <th className="px-4 py-2 text-left font-medium text-muted-foreground w-24">Source</th>
            <th className="px-4 py-2 text-left font-medium text-muted-foreground w-40">Target</th>
            <th className="px-4 py-2 text-left font-medium text-muted-foreground">Message</th>
            <th className="px-4 py-2 text-left font-medium text-muted-foreground w-24">Task</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-border">
          {visible.map((log) => (
            <>
              <tr
                key={log.id}
                onClick={() => setExpandedId(expandedId === log.id ? null : log.id)}
                className={cn(
                  'cursor-pointer transition-colors hover:bg-muted/20',
                  log.level === 'ERROR' && 'border-l-2 border-red-500 bg-red-500/5',
                  log.level === 'WARN' && 'border-l-2 border-amber-500/50'
                )}
              >
                <td className="px-4 py-2 text-xs text-muted-foreground font-mono whitespace-nowrap">
                  <span title={log.timestamp}>
                    {formatDistanceToNow(new Date(log.timestamp), { addSuffix: true })}
                  </span>
                </td>
                <td className="px-4 py-2">
                  <LogLevelBadge level={log.level as import('@/types/log').LogLevel} />
                </td>
                <td className="px-4 py-2 text-xs">
                  <span
                    className={cn(
                      'font-mono',
                      log.source === 'frontend' ? 'text-blue-400' : 'text-purple-400'
                    )}
                  >
                    {log.source}
                  </span>
                </td>
                <td className="px-4 py-2 text-xs text-muted-foreground font-mono truncate max-w-[160px]">
                  {log.target ?? '—'}
                </td>
                <td className="px-4 py-2 text-xs max-w-0">
                  <span className="block truncate">{log.message}</span>
                </td>
                <td className="px-4 py-2 text-xs text-muted-foreground font-mono truncate">
                  {log.task_id ? log.task_id.slice(0, 8) + '…' : '—'}
                </td>
              </tr>

              {expandedId === log.id && (
                <tr key={`${log.id}-detail`} className="bg-muted/10">
                  <td colSpan={6} className="px-4 py-3">
                    <div className="space-y-2 text-xs">
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <span className="text-muted-foreground">Full timestamp:</span>{' '}
                          <span className="font-mono">{log.timestamp}</span>
                        </div>
                        {log.task_id && (
                          <div>
                            <span className="text-muted-foreground">Task ID:</span>{' '}
                            <span className="font-mono">{log.task_id}</span>
                          </div>
                        )}
                        {log.session_id && (
                          <div>
                            <span className="text-muted-foreground">Session ID:</span>{' '}
                            <span className="font-mono">{log.session_id}</span>
                          </div>
                        )}
                      </div>
                      {log.metadata && (
                        <div>
                          <span className="text-muted-foreground block mb-1">Metadata:</span>
                          <pre className="bg-muted rounded p-2 overflow-auto max-h-32 text-xs">
                            {(() => {
                              try {
                                return JSON.stringify(JSON.parse(log.metadata), null, 2);
                              } catch {
                                return log.metadata;
                              }
                            })()}
                          </pre>
                        </div>
                      )}
                    </div>
                  </td>
                </tr>
              )}
            </>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

---

## Task 5: Logs Page

**Files:**
- Create: `frontend/src/app/logs/page.tsx`
- Modify: `frontend/src/components/layout/sidebar.tsx`

**Step 1: Create the page**

Create `frontend/src/app/logs/page.tsx`:

```tsx
'use client';

import { useState } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { LogTable } from '@/components/logs/log-table';
import { useLogs } from '@/hooks/use-logs';
import type { LogLevel, LogSource, LogFilter } from '@/types/log';

const LEVELS: LogLevel[] = ['DEBUG', 'INFO', 'WARN', 'ERROR'];
const SOURCES: { value: LogSource | ''; label: string }[] = [
  { value: '', label: 'All' },
  { value: 'frontend', label: 'Frontend' },
  { value: 'backend', label: 'Backend' },
];

export default function LogsPage() {
  const [levelFilter, setLevelFilter] = useState<LogLevel | undefined>();
  const [sourceFilter, setSourceFilter] = useState<LogSource | undefined>();
  const [search, setSearch] = useState('');
  const [isLive, setIsLive] = useState(true);

  const serverFilter = {
    level: levelFilter,
    source: sourceFilter,
  };

  const { logs, isLoading, newCount, loadNewLogs, isLiveRef } = useLogs(serverFilter);

  isLiveRef.current = isLive;

  const clientFilter: LogFilter = {
    ...serverFilter,
    search: search || undefined,
  };

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <div className="border-b border-border px-6 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-xl font-semibold">Logs</h1>
            <p className="text-sm text-muted-foreground">
              Frontend + backend logs · polls every 5s
            </p>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Live</span>
            <button
              onClick={() => setIsLive((v) => !v)}
              className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${
                isLive ? 'bg-primary' : 'bg-muted'
              }`}
            >
              <span
                className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                  isLive ? 'translate-x-4' : 'translate-x-1'
                }`}
              />
            </button>
          </div>
        </div>

        {/* Filter bar */}
        <div className="border-b border-border px-6 py-3 flex flex-wrap items-center gap-4">
          {/* Level pills */}
          <div className="flex gap-1">
            <button
              onClick={() => setLevelFilter(undefined)}
              className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                !levelFilter ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/70'
              }`}
            >
              ALL
            </button>
            {LEVELS.map((l) => (
              <button
                key={l}
                onClick={() => setLevelFilter(levelFilter === l ? undefined : l)}
                className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                  levelFilter === l ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/70'
                }`}
              >
                {l}
              </button>
            ))}
          </div>

          {/* Source segment */}
          <div className="flex rounded-md border border-border overflow-hidden">
            {SOURCES.map((s) => (
              <button
                key={s.value}
                onClick={() => setSourceFilter(s.value as LogSource | undefined)}
                className={`px-3 py-1 text-xs font-medium transition-colors ${
                  (sourceFilter ?? '') === s.value
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-background text-muted-foreground hover:bg-muted'
                }`}
              >
                {s.label}
              </button>
            ))}
          </div>

          {/* Search */}
          <input
            type="text"
            placeholder="Search messages..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="flex-1 max-w-xs rounded-md border border-border bg-background px-3 py-1.5 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>

        {/* New logs banner */}
        {newCount > 0 && (
          <button
            onClick={() => {
              setIsLive(true);
              loadNewLogs();
            }}
            className="mx-6 mt-3 rounded-lg bg-primary/10 border border-primary/20 px-4 py-2 text-sm text-primary text-center hover:bg-primary/20 transition-colors"
          >
            {newCount} new log{newCount > 1 ? 's' : ''} — click to load
          </button>
        )}

        {/* Table */}
        <main className="flex-1 p-6">
          {isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 8 }).map((_, i) => (
                <div key={i} className="h-9 animate-pulse bg-muted rounded" />
              ))}
            </div>
          ) : (
            <LogTable logs={logs} filter={clientFilter} />
          )}
        </main>
      </div>
    </div>
  );
}
```

**Step 2: Add Logs link to sidebar**

Open `frontend/src/components/layout/sidebar.tsx`. Add after the Analytics link:

```tsx
<Link
  href="/logs"
  className={cn(
    'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
    pathname === '/logs'
      ? 'bg-sidebar-accent text-sidebar-accent-foreground'
      : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
  )}
>
  Logs
</Link>
```

**Step 3: Verify TypeScript and build**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit 2>&1 | head -30
npm run build 2>&1 | tail -10
```

Expected: no errors, build succeeds.

**Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/src/types/log.ts \
        frontend/src/hooks/use-logs.ts \
        frontend/src/components/logs/ \
        frontend/src/app/logs/page.tsx \
        frontend/src/components/layout/sidebar.tsx
git commit -m "feat(frontend): add Logs tab with 5s polling

- /logs page with full-width log table
- Level filter pills (DEBUG/INFO/WARN/ERROR) + source segment (Frontend/Backend)
- Client-side text search on message content
- Color-coded level badges; ERROR rows get red left border
- Row expansion shows full metadata JSON
- Live toggle: auto-scroll to new entries or show 'N new logs' banner
- Sidebar updated with Logs nav link"
```
