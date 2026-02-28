'use client';

import { cn } from '@/lib/utils';
import type { LogEntry, LogLevel } from '@/types/log';

const LEVEL_CONFIG: Record<LogLevel, { label: string; bg: string; text: string; activeBg: string }> = {
  DEBUG: { label: 'DEBUG', bg: 'bg-muted/50', text: 'text-muted-foreground', activeBg: 'bg-muted' },
  INFO:  { label: 'INFO',  bg: 'bg-blue-500/10',  text: 'text-blue-400',  activeBg: 'bg-blue-500/25' },
  WARN:  { label: 'WARN',  bg: 'bg-amber-500/10', text: 'text-amber-400', activeBg: 'bg-amber-500/25' },
  ERROR: { label: 'ERROR', bg: 'bg-red-500/10',   text: 'text-red-400',   activeBg: 'bg-red-500/25' },
};

const LEVELS: LogLevel[] = ['ERROR', 'WARN', 'INFO', 'DEBUG'];

interface Props {
  logs: LogEntry[];
  activeLevel?: LogLevel;
  onLevelClick: (level: LogLevel | undefined) => void;
}

export function LogStatsBar({ logs, activeLevel, onLevelClick }: Props) {
  const counts = logs.reduce<Record<LogLevel, number>>(
    (acc, log) => { acc[log.level as LogLevel] = (acc[log.level as LogLevel] ?? 0) + 1; return acc; },
    { DEBUG: 0, INFO: 0, WARN: 0, ERROR: 0 }
  );

  return (
    <div className="flex items-center gap-2 flex-wrap">
      {LEVELS.map((level) => {
        const cfg = LEVEL_CONFIG[level];
        const isActive = activeLevel === level;
        return (
          <button
            key={level}
            onClick={() => onLevelClick(isActive ? undefined : level)}
            className={cn(
              'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border',
              isActive
                ? `${cfg.activeBg} ${cfg.text} border-current/30`
                : `${cfg.bg} ${cfg.text} border-transparent hover:border-current/20`
            )}
          >
            <span className="font-mono">{cfg.label}</span>
            <span className="font-bold tabular-nums">{counts[level]}</span>
          </button>
        );
      })}
      <span className="text-xs text-muted-foreground ml-1">
        {logs.length} total loaded
      </span>
    </div>
  );
}
