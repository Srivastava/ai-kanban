'use client';

import { useState, useEffect } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { LogTable } from '@/components/logs/log-table';
import { LogStatsBar } from '@/components/logs/log-stats-bar';
import { RecentErrorsPanel } from '@/components/logs/recent-errors-panel';
import { LogRateChart } from '@/components/logs/log-rate-chart';
import { useLogs } from '@/hooks/use-logs';
import { logger } from '@/lib/logger';
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
  const [taskIdFilter, setTaskIdFilter] = useState('');
  const [isLive, setIsLive] = useState(true);

  useEffect(() => {
    logger.info('LogsPage mounted');
    return () => {
      logger.debug('LogsPage unmounted');
    };
  }, []);

  useEffect(() => {
    logger.debug('LogsPage: filter changed', { levelFilter, sourceFilter, search, isLive });
  }, [levelFilter, sourceFilter, search, isLive]);

  // UUID pattern — only pass to server when input looks complete
  const isUuid = (s: string) => /^[0-9a-f-]{36}$/i.test(s.trim());

  const serverFilter = {
    level: levelFilter,
    source: sourceFilter,
    task_id: isUuid(taskIdFilter) ? taskIdFilter.trim() : undefined,
  };

  const { logs, isLoading, newCount, loadNewLogs, isLiveRef } = useLogs(serverFilter);

  // Unfiltered query dedicated to powering the debug widgets so they always
  // reflect the full log set regardless of the active level/source filter.
  const { logs: allLogs } = useLogs({});

  isLiveRef.current = isLive;

  const clientFilter: LogFilter = {
    ...serverFilter,
    search: search || undefined,
    task_id: serverFilter.task_id,
  };

  useEffect(() => {
    logger.debug('LogsPage: logs updated', { count: logs.length, newCount, isLoading });
  }, [logs.length, newCount, isLoading]);

  const handleLevelFilterChange = (level: LogLevel | undefined) => {
    logger.debug('LogsPage: level filter changed', { newLevel: level, previousLevel: levelFilter });
    setLevelFilter(level);
  };

  const handleSourceFilterChange = (source: LogSource | undefined) => {
    logger.debug('LogsPage: source filter changed', { newSource: source, previousSource: sourceFilter });
    setSourceFilter(source);
  };

  const handleLiveToggle = () => {
    const newValue = !isLive;
    logger.debug('LogsPage: live toggle changed', { isLive: newValue });
    setIsLive(newValue);
  };

  const handleLoadNewLogs = () => {
    logger.debug('LogsPage: loading new logs', { newCount });
    setIsLive(true);
    loadNewLogs();
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
              onClick={handleLiveToggle}
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
              onClick={() => handleLevelFilterChange(undefined)}
              className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                !levelFilter ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/70'
              }`}
            >
              ALL
            </button>
            {LEVELS.map((l) => (
              <button
                key={l}
                onClick={() => handleLevelFilterChange(levelFilter === l ? undefined : l)}
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
                onClick={() => handleSourceFilterChange(s.value === '' ? undefined : s.value as LogSource)}
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

          {/* Task ID filter */}
          <div className="relative flex items-center">
            <input
              type="text"
              placeholder="Task ID (UUID)..."
              value={taskIdFilter}
              onChange={(e) => setTaskIdFilter(e.target.value)}
              className={`w-72 rounded-md border bg-background px-3 py-1.5 pr-7 text-sm font-mono placeholder:text-muted-foreground placeholder:font-sans focus:outline-none focus:ring-1 focus:ring-ring transition-colors ${
                taskIdFilter && isUuid(taskIdFilter)
                  ? 'border-primary ring-1 ring-primary/30'
                  : taskIdFilter
                  ? 'border-amber-400'
                  : 'border-border'
              }`}
            />
            {taskIdFilter && (
              <button
                onClick={() => setTaskIdFilter('')}
                className="absolute right-2 text-muted-foreground hover:text-foreground text-xs"
                title="Clear task filter"
              >
                ✕
              </button>
            )}
          </div>
        </div>

        {/* New logs banner */}
        {newCount > 0 && (
          <button
            onClick={handleLoadNewLogs}
            className="mx-6 mt-3 rounded-lg bg-primary/10 border border-primary/20 px-4 py-2 text-sm text-primary text-center hover:bg-primary/20 transition-colors"
          >
            {newCount} new log{newCount > 1 ? 's' : ''} — click to load
          </button>
        )}

        <main className="flex-1 p-6 space-y-4">
          {/* Stats bar */}
          <LogStatsBar
            logs={allLogs}
            activeLevel={levelFilter}
            onLevelClick={handleLevelFilterChange}
          />

          {/* Log rate sparkline */}
          <LogRateChart logs={allLogs} />

          {/* Recent errors */}
          <RecentErrorsPanel logs={allLogs} />

          {/* Table */}
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
