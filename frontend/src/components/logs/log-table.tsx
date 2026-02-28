'use client';

import { useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { LogLevelBadge } from './log-level-badge';
import { cn } from '@/lib/utils';
import type { LogEntry, LogFilter, LogLevel } from '@/types/log';

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
                  <LogLevelBadge level={log.level as LogLevel} />
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
