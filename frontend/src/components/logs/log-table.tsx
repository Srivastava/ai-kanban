'use client';

import { useState, useCallback, Fragment, useEffect } from 'react';
import { LogLevelBadge } from './log-level-badge';
import { cn } from '@/lib/utils';
import type { LogEntry, LogFilter, LogLevel } from '@/types/log';

const PAGE_SIZE = 50;

type SortKey = 'timestamp' | 'level' | 'source' | 'message';
type SortDir = 'asc' | 'desc';

const TZ = 'America/Los_Angeles';

function formatLogTime(iso: string): string {
  const d = new Date(iso);
  return new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
    hour12: false,
  }).format(d);
}

function formatFullTime(iso: string): string {
  const d = new Date(iso);
  return new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
    hour12: false,
    timeZoneName: 'short',
  }).format(d);
}

const LEVEL_ORDER: Record<LogLevel, number> = { DEBUG: 0, INFO: 1, WARN: 2, ERROR: 3 };

interface Props {
  logs: LogEntry[];
  filter: LogFilter;
}

function SortIcon({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) {
    return <span className="ml-1 opacity-25 text-[10px]">⇅</span>;
  }
  return <span className="ml-1 text-[10px]">{dir === 'asc' ? '↑' : '↓'}</span>;
}

export function LogTable({ logs, filter }: Props) {
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>('timestamp');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [page, setPage] = useState(0);

  const handleSort = useCallback((key: SortKey) => {
    setSortKey((prev) => {
      if (prev === key) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
        return prev;
      }
      setSortDir('desc');
      return key;
    });
    setPage(0);
  }, []);

  // Reset to page 0 when filter or logs change
  useEffect(() => { setPage(0); }, [filter.level, filter.source, filter.search, logs.length]);

  const visible = logs
    .filter((log) => {
      if (filter.level && log.level !== filter.level) return false;
      if (filter.source && log.source !== filter.source) return false;
      if (filter.search) {
        const q = filter.search.toLowerCase();
        const inMessage = log.message.toLowerCase().includes(q);
        const inMetadata = log.metadata ? log.metadata.toLowerCase().includes(q) : false;
        const inTaskId = log.task_id ? log.task_id.toLowerCase().includes(q) : false;
        const inTarget = log.target ? log.target.toLowerCase().includes(q) : false;
        if (!inMessage && !inMetadata && !inTaskId && !inTarget) return false;
      }
      return true;
    })
    .sort((a, b) => {
      let cmp = 0;
      if (sortKey === 'timestamp') cmp = a.timestamp.localeCompare(b.timestamp);
      else if (sortKey === 'level') cmp = LEVEL_ORDER[a.level as LogLevel] - LEVEL_ORDER[b.level as LogLevel];
      else if (sortKey === 'source') cmp = a.source.localeCompare(b.source);
      else if (sortKey === 'message') cmp = a.message.localeCompare(b.message);
      return sortDir === 'asc' ? cmp : -cmp;
    });

  const totalPages = Math.max(1, Math.ceil(visible.length / PAGE_SIZE));
  const safePage = Math.min(page, totalPages - 1);
  const pageRows = visible.slice(safePage * PAGE_SIZE, (safePage + 1) * PAGE_SIZE);

  if (visible.length === 0) {
    return (
      <div className="flex items-center justify-center h-48 rounded-xl border border-border">
        <p className="text-muted-foreground text-sm">No logs match the current filters</p>
      </div>
    );
  }

  const thClass =
    'px-3 py-2.5 text-left text-xs font-semibold text-muted-foreground select-none cursor-pointer hover:text-foreground transition-colors whitespace-nowrap';
  const thStaticClass =
    'px-3 py-2.5 text-left text-xs font-semibold text-muted-foreground whitespace-nowrap';

  return (
    <div className="rounded-xl border border-border overflow-hidden flex flex-col">
      <div className="overflow-auto max-h-[calc(100vh-300px)]">
        <table className="w-full text-xs border-collapse">
          <thead className="sticky top-0 z-10">
            <tr className="border-b border-border bg-muted/60 backdrop-blur-sm">
              <th className={thClass} style={{ width: 110 }} onClick={() => handleSort('timestamp')}>
                Time <SortIcon active={sortKey === 'timestamp'} dir={sortDir} />
              </th>
              <th className={thClass} style={{ width: 72 }} onClick={() => handleSort('level')}>
                Level <SortIcon active={sortKey === 'level'} dir={sortDir} />
              </th>
              <th className={thClass} style={{ width: 82 }} onClick={() => handleSort('source')}>
                Source <SortIcon active={sortKey === 'source'} dir={sortDir} />
              </th>
              <th className={thStaticClass} style={{ width: 150 }}>
                Target
              </th>
              <th className={thClass} onClick={() => handleSort('message')}>
                Message <SortIcon active={sortKey === 'message'} dir={sortDir} />
              </th>
              <th className={thStaticClass} style={{ width: 88 }}>
                Task
              </th>
              <th className={thStaticClass} style={{ width: 88 }}>
                Session
              </th>
            </tr>
          </thead>
          <tbody>
            {pageRows.map((log, i) => (
              <Fragment key={log.id}>
                <tr
                  onClick={() => setExpandedId(expandedId === log.id ? null : log.id)}
                  className={cn(
                    'cursor-pointer transition-colors border-b border-border/40',
                    i % 2 === 0 ? 'bg-background' : 'bg-muted/10',
                    'hover:bg-primary/5',
                    log.level === 'ERROR' &&
                      'border-l-2 !border-l-red-500 bg-red-500/5 hover:bg-red-500/10',
                    log.level === 'WARN' &&
                      'border-l-2 !border-l-amber-400 bg-amber-500/5 hover:bg-amber-500/10',
                    expandedId === log.id && '!bg-primary/8'
                  )}
                >
                  <td className="px-3 py-1.5 font-mono text-muted-foreground whitespace-nowrap" title={formatFullTime(log.timestamp)}>
                    {formatLogTime(log.timestamp)}
                  </td>
                  <td className="px-3 py-1.5">
                    <LogLevelBadge level={log.level as LogLevel} />
                  </td>
                  <td className="px-3 py-1.5">
                    <span
                      className={cn(
                        'font-mono font-semibold',
                        log.source === 'frontend' ? 'text-blue-400' : 'text-purple-400'
                      )}
                    >
                      {log.source}
                    </span>
                  </td>
                  <td
                    className="px-3 py-1.5 font-mono text-muted-foreground truncate"
                    style={{ maxWidth: 150 }}
                    title={log.target ?? undefined}
                  >
                    {log.target ?? '—'}
                  </td>
                  <td className="px-3 py-1.5 max-w-0">
                    <span
                      className="block truncate text-foreground/90"
                      title={log.message}
                    >
                      {log.message}
                    </span>
                  </td>
                  <td className="px-3 py-1.5 font-mono text-muted-foreground whitespace-nowrap">
                    {log.task_id ? (
                      <button
                        className="hover:text-foreground transition-colors cursor-pointer"
                        title={`Click to copy: ${log.task_id}`}
                        onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(log.task_id!); }}
                      >
                        {log.task_id.slice(0, 8)}…
                      </button>
                    ) : '—'}
                  </td>
                  <td className="px-3 py-1.5 font-mono text-muted-foreground whitespace-nowrap">
                    {log.session_id ? (
                      <button
                        className="hover:text-foreground transition-colors cursor-pointer"
                        title={`Click to copy: ${log.session_id}`}
                        onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(log.session_id!); }}
                      >
                        {log.session_id.slice(0, 8)}…
                      </button>
                    ) : '—'}
                  </td>
                </tr>

                {expandedId === log.id && (
                  <tr>
                    <td
                      colSpan={7}
                      className={cn(
                        'px-5 py-3 border-b border-border/40',
                        'bg-muted/25 border-l-2',
                        log.level === 'ERROR' ? 'border-l-red-500' : 'border-l-primary/40'
                      )}
                    >
                      <div className="space-y-2 text-xs">
                        <div className="flex flex-wrap gap-x-6 gap-y-1">
                          <div>
                            <span className="text-muted-foreground">Timestamp: </span>
                            <span className="font-mono">{formatFullTime(log.timestamp)}</span>
                          </div>
                          {log.task_id && (
                            <div>
                              <span className="text-muted-foreground">Task ID: </span>
                              <span className="font-mono">{log.task_id}</span>
                            </div>
                          )}
                          {log.session_id && (
                            <div>
                              <span className="text-muted-foreground">Session ID: </span>
                              <span className="font-mono">{log.session_id}</span>
                            </div>
                          )}
                        </div>
                        {log.target && (
                          <div>
                            <span className="text-muted-foreground">Target: </span>
                            <span className="font-mono">{log.target}</span>
                          </div>
                        )}
                        {log.metadata && (
                          <div>
                            <span className="text-muted-foreground block mb-1">Metadata:</span>
                            <pre className="bg-muted rounded-lg p-2.5 overflow-auto max-h-48 text-xs leading-relaxed font-mono">
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
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>
      <div className="border-t border-border px-4 py-2 bg-muted/20 flex items-center justify-between shrink-0 gap-4">
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          <span className="font-medium text-foreground">{visible.length}</span> log
          {visible.length !== 1 ? 's' : ''}
          {logs.length !== visible.length && (
            <span> &nbsp;·&nbsp; {logs.length} loaded</span>
          )}
          &nbsp;·&nbsp; sorted by <span className="font-medium text-foreground">{sortKey}</span>{' '}
          {sortDir === 'asc' ? '↑' : '↓'}
        </span>

        {totalPages > 1 && (
          <div className="flex items-center gap-1 shrink-0">
            <button
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              disabled={safePage === 0}
              className="px-2 py-0.5 text-xs rounded border border-border disabled:opacity-30 hover:bg-muted/60 transition-colors"
            >
              ‹
            </button>
            {Array.from({ length: totalPages }, (_, i) => {
              // Show first, last, current ±1, and ellipsis
              const show =
                i === 0 || i === totalPages - 1 || Math.abs(i - safePage) <= 1;
              if (!show) {
                const prev = i - 1;
                const prevShow =
                  prev === 0 || prev === totalPages - 1 || Math.abs(prev - safePage) <= 1;
                return prevShow ? (
                  <span key={`ellipsis-${i}`} className="text-xs text-muted-foreground px-1">…</span>
                ) : null;
              }
              return (
                <button
                  key={i}
                  onClick={() => setPage(i)}
                  className={cn(
                    'min-w-[24px] px-1.5 py-0.5 text-xs rounded border transition-colors',
                    i === safePage
                      ? 'border-primary bg-primary/10 text-primary font-medium'
                      : 'border-border hover:bg-muted/60'
                  )}
                >
                  {i + 1}
                </button>
              );
            })}
            <button
              onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
              disabled={safePage === totalPages - 1}
              className="px-2 py-0.5 text-xs rounded border border-border disabled:opacity-30 hover:bg-muted/60 transition-colors"
            >
              ›
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
