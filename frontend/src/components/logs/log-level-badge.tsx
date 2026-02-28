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
