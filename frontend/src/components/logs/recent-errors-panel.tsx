'use client';

import { useState } from 'react';
import { ChevronDown, AlertCircle } from 'lucide-react';
import type { LogEntry } from '@/types/log';

const TZ = 'America/Los_Angeles';
function fmtTime(iso: string) {
  return new Intl.DateTimeFormat('en-US', {
    timeZone: TZ, hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
  }).format(new Date(iso));
}

interface Props {
  logs: LogEntry[];          // full unfiltered log list
  onJumpToLog?: (id: number) => void;
}

export function RecentErrorsPanel({ logs, onJumpToLog }: Props) {
  const [open, setOpen] = useState(true);

  const errors = logs
    .filter((l) => l.level === 'ERROR')
    .sort((a, b) => b.timestamp.localeCompare(a.timestamp))
    .slice(0, 5);

  if (errors.length === 0) return null;

  return (
    <div className="rounded-xl border border-red-500/30 bg-red-500/5 overflow-hidden">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-4 py-2.5 text-sm font-medium text-red-400 hover:bg-red-500/10 transition-colors"
      >
        <AlertCircle className="h-4 w-4 shrink-0" />
        <span>Recent Errors ({errors.length})</span>
        <ChevronDown className={`h-3.5 w-3.5 ml-auto transition-transform duration-300 ease-out motion-reduce:transition-none ${open ? '' : '-rotate-90'}`} />
      </button>

      <div className="collapse-grid" style={{ gridTemplateRows: open ? '1fr' : '0fr' }}>
        <div className="overflow-hidden">
        <div className="divide-y divide-red-500/10">
          {errors.map((log, i) => (
            <button
              key={log.id}
              onClick={() => onJumpToLog?.(log.id)}
              className="w-full flex items-start gap-3 px-4 py-2 text-xs hover:bg-red-500/10 transition-colors text-left motion-safe:animate-fade-in-up"
              style={{ animationDelay: `${i * 40}ms` }}
            >
              <span className="font-mono text-muted-foreground shrink-0 mt-0.5">{fmtTime(log.timestamp)}</span>
              <span className="font-mono text-purple-400 shrink-0 mt-0.5">{log.source}</span>
              <span className="text-foreground/90 truncate">{log.message}</span>
              {log.task_id && (
                <span className="font-mono text-muted-foreground shrink-0 ml-auto">
                  {log.task_id.slice(0, 8)}
                </span>
              )}
            </button>
          ))}
        </div>
        </div>
      </div>
    </div>
  );
}
