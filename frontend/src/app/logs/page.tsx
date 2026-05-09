'use client';

import { useState, useEffect } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { LogTable } from '@/components/logs/log-table';
import { LogStatsBar } from '@/components/logs/log-stats-bar';
import { RecentErrorsPanel } from '@/components/logs/recent-errors-panel';
import { LogRateChart } from '@/components/logs/log-rate-chart';
import { SessionLifecyclePanel } from '@/components/logs/session-lifecycle-panel';
import { SessionDiagnosticsPanel } from '@/components/logs/session-diagnostics-panel';
import { ActiveSessionsPanel } from '@/components/logs/active-sessions-panel';
import { LogContextBreakdown } from '@/components/logs/log-context-breakdown';
import { useLogs } from '@/hooks/use-logs';
import { apiClient } from '@/lib/api-client';
import { logger } from '@/lib/logger';
import type { LogLevel, LogSource, LogFilter } from '@/types/log';
import type { Session } from '@/types/session';

const LEVELS: LogLevel[] = ['DEBUG', 'INFO', 'WARN', 'ERROR'];

const LEVEL_ACTIVE_CLASSES: Record<LogLevel, string> = {
  DEBUG: 'bg-slate-500/15 text-slate-700 dark:text-slate-300 border border-slate-500/30',
  INFO:  'bg-blue-500/15  text-blue-700  dark:text-blue-400  border border-blue-500/30',
  WARN:  'bg-amber-500/15 text-amber-700 dark:text-amber-400 border border-amber-500/30',
  ERROR: 'bg-red-500/15   text-red-700   dark:text-red-400   border border-red-500/30',
};
const SOURCES: { value: LogSource | ''; label: string }[] = [
  { value: '', label: 'All' },
  { value: 'frontend', label: 'Frontend' },
  { value: 'backend', label: 'Backend' },
];

const isUuid = (s: string) => /^[0-9a-f-]{36}$/i.test(s.trim());

export default function LogsPage() {
  const [levelFilter, setLevelFilter] = useState<LogLevel | undefined>();
  const [sourceFilter, setSourceFilter] = useState<LogSource | undefined>();
  const [search, setSearch] = useState('');
  const [taskIdFilter, setTaskIdFilter] = useState('');
  const [sessionIdFilter, setSessionIdFilter] = useState('');
  const [claudeSessionIdFilter, setClaudeSessionIdFilter] = useState('');
  const [resolvedSessionId, setResolvedSessionId] = useState<string | null>(null);
  const [isLive, setIsLive] = useState(true);

  useEffect(() => {
    logger.info('LogsPage mounted');
  }, []);

  // Resolve claude_session_id → internal session_id for log filtering
  useEffect(() => {
    if (!claudeSessionIdFilter || !isUuid(claudeSessionIdFilter)) {
      setResolvedSessionId(null);
      return;
    }
    apiClient<Session>(`/api/sessions/by-claude-id/${claudeSessionIdFilter.trim()}`)
      .then((s) => setResolvedSessionId(s.id))
      .catch(() => setResolvedSessionId(null));
  }, [claudeSessionIdFilter]);

  // Effective internal session_id: explicit input takes priority, then resolved from claude id
  const effectiveSessionId = isUuid(sessionIdFilter)
    ? sessionIdFilter.trim()
    : resolvedSessionId ?? undefined;

  const serverFilter = {
    task_id: isUuid(taskIdFilter) ? taskIdFilter.trim() : undefined,
    session_id: effectiveSessionId,
  };

  const { logs, isLoading, newCount, loadNewLogs, isLiveRef } = useLogs(serverFilter);
  const { logs: allLogs } = useLogs({});

  useEffect(() => {
    isLiveRef.current = isLive;
  }, [isLive, isLiveRef]);

  const clientFilter: LogFilter = {
    level: levelFilter,
    source: sourceFilter,
    task_id: serverFilter.task_id,
    session_id: serverFilter.session_id,
    search: search || undefined,
  };

  // Shared filter callbacks used by panels
  const handleTaskClick = (id: string) => {
    setTaskIdFilter(id);
  };
  const handleSessionClick = (id: string) => {
    setSessionIdFilter(id);
    setClaudeSessionIdFilter('');
  };
  const handleClaudeSessionClick = (id: string) => {
    setClaudeSessionIdFilter(id);
    setSessionIdFilter('');
  };

  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <div className="border-b border-border px-4 sm:px-6 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-black tracking-tighter leading-none">Logs</h1>
            <p className="text-xs text-red-600 dark:text-red-400 mt-0.5 font-medium">Frontend + backend · live stream</p>
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
        <div className="border-b border-border px-4 sm:px-6 py-3 flex flex-wrap items-center gap-3">
          {/* Level pills */}
          <div className="flex gap-1">
            <button
              onClick={() => setLevelFilter(undefined)}
              className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                !levelFilter
                  ? 'bg-foreground/12 text-foreground border border-foreground/20'
                  : 'bg-muted text-muted-foreground hover:bg-muted/70'
              }`}
            >
              ALL
            </button>
            {LEVELS.map((l) => (
              <button
                key={l}
                onClick={() => setLevelFilter(levelFilter === l ? undefined : l)}
                className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                  levelFilter === l
                    ? LEVEL_ACTIVE_CLASSES[l]
                    : 'bg-muted text-muted-foreground hover:bg-muted/70'
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
                onClick={() => setSourceFilter(s.value === '' ? undefined : s.value as LogSource)}
                className={`px-3 py-1 text-xs font-medium transition-colors ${
                  (sourceFilter ?? '') === s.value
                    ? 'bg-primary/15 text-primary font-semibold'
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

          {/* Task ID filter */}
          <FilterInput
            value={taskIdFilter}
            onChange={setTaskIdFilter}
            placeholder="Task ID…"
            activeColor="primary"
          />

          {/* Internal Session ID filter */}
          <FilterInput
            value={sessionIdFilter}
            onChange={(v) => { setSessionIdFilter(v); setClaudeSessionIdFilter(''); }}
            placeholder="Session ID (internal)…"
            activeColor="violet"
          />

          {/* Claude Session ID filter */}
          <FilterInput
            value={claudeSessionIdFilter}
            onChange={(v) => { setClaudeSessionIdFilter(v); setSessionIdFilter(''); }}
            placeholder="Claude Session ID…"
            activeColor="violet"
            extraRight={
              claudeSessionIdFilter && isUuid(claudeSessionIdFilter) ? (
                resolvedSessionId
                  ? <span className="absolute right-7 text-[10px] text-green-500 pointer-events-none" title="Resolved">✓</span>
                  : <span className="absolute right-7 text-[10px] text-amber-500 pointer-events-none" title="Not found">?</span>
              ) : null
            }
          />

          {/* Active filters badge */}
          {(taskIdFilter || sessionIdFilter || claudeSessionIdFilter) && (
            <button
              onClick={() => { setTaskIdFilter(''); setSessionIdFilter(''); setClaudeSessionIdFilter(''); }}
              className="text-xs text-muted-foreground hover:text-foreground px-2 py-1 rounded border border-border hover:bg-muted transition-colors"
            >
              Clear filters ✕
            </button>
          )}
        </div>

        {/* New logs banner */}
        {newCount > 0 && (
          <button
            onClick={() => { setIsLive(true); loadNewLogs(); }}
            className="mx-6 mt-3 rounded-lg bg-primary/10 border border-primary/20 px-4 py-2 text-sm text-primary text-center hover:bg-primary/20 active:scale-[0.98] transition-[colors,transform] duration-150 motion-safe:animate-fade-in-down"
          >
            ↓ {newCount} new log{newCount > 1 ? 's' : ''} — click to load
          </button>
        )}

        <main className="flex-1 p-4 sm:p-6 pb-20 md:pb-6 space-y-4">
          {/* Stats bar — always visible */}
          <LogStatsBar logs={allLogs} activeLevel={levelFilter} onLevelClick={setLevelFilter} />

          {/* Log rate sparkline — always visible */}
          <LogRateChart logs={allLogs} />

          {/* Recent errors — visible when errors exist */}
          <RecentErrorsPanel logs={allLogs} />

          {/* Recent Sessions — always visible, from analytics */}
          <ActiveSessionsPanel
            onSessionClick={handleSessionClick}
            activeSessionId={sessionIdFilter && isUuid(sessionIdFilter) ? sessionIdFilter.trim() : undefined}
          />

          {/* Log context breakdown — always visible, derived from loaded logs */}
          <LogContextBreakdown
            logs={serverFilter.task_id ? logs : allLogs}
            onTaskClick={handleTaskClick}
            onSessionClick={handleSessionClick}
            activeTaskId={serverFilter.task_id}
            activeSessionId={serverFilter.session_id}
          />

          {/* Session Diagnostics — shows failed/stuck sessions with error context */}
          <SessionDiagnosticsPanel
            onSessionClick={handleSessionClick}
            onTaskClick={handleTaskClick}
            taskId={serverFilter.task_id}
          />

          {/* Session Lifecycle — always visible with task picker */}
          <SessionLifecyclePanel
            taskId={serverFilter.task_id ?? null}
            activeSessionId={sessionIdFilter && isUuid(sessionIdFilter) ? sessionIdFilter.trim() : undefined}
            activeClaudeSessionId={claudeSessionIdFilter && isUuid(claudeSessionIdFilter) ? claudeSessionIdFilter.trim() : undefined}
            onSessionClick={handleSessionClick}
            onClaudeSessionClick={handleClaudeSessionClick}
            onTaskSelect={(id) => setTaskIdFilter(id)}
          />

          {/* Log table */}
          {isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 8 }).map((_, i) => (
                <div key={i} className="h-9 bg-muted rounded animate-shimmer" />
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

function FilterInput({
  value,
  onChange,
  placeholder,
  activeColor,
  extraRight,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  activeColor: 'primary' | 'violet';
  extraRight?: React.ReactNode;
}) {
  const activeClass =
    activeColor === 'primary'
      ? 'border-primary ring-1 ring-primary/30'
      : 'border-violet-500 ring-1 ring-violet-500/30';

  return (
    <div className="relative flex items-center">
      <input
        type="text"
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`w-52 rounded-md border bg-background px-3 py-1.5 pr-7 text-sm font-mono placeholder:text-muted-foreground placeholder:font-sans focus:outline-none focus:ring-1 focus:ring-ring transition-colors ${
          value && isUuid(value) ? activeClass : value ? 'border-amber-400' : 'border-border'
        }`}
      />
      {extraRight}
      {value && (
        <button
          onClick={() => onChange('')}
          className="absolute right-2 text-muted-foreground hover:text-foreground text-xs"
        >
          ✕
        </button>
      )}
    </div>
  );
}
