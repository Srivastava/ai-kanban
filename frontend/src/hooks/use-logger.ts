'use client';

import { useMemo } from 'react';
import { logger } from '@/lib/logger';
import type { ContextLogger } from '@/lib/logger';

interface UseLoggerOptions {
  target?: string;
  taskId?: string;
  sessionId?: string;
}

export function useLogger(options: UseLoggerOptions = {}): ContextLogger {
  return useMemo(
    () =>
      logger.withContext({
        target: options.target,
        task_id: options.taskId,
        session_id: options.sessionId,
      }),
    [options.target, options.taskId, options.sessionId]
  );
}
