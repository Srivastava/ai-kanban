'use client';

import { useState, useCallback, useRef, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import { logger } from '@/lib/logger';
import type { LogEntry, LogFilter } from '@/types/log';

const POLL_INTERVAL = 5_000;

export function useLogs(filter: Omit<LogFilter, 'search'> = {}) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [newCount, setNewCount] = useState(0);
  const lastTimestampRef = useRef<string | null>(null);
  const isLiveRef = useRef(true);

  logger.debug('useLogs hook called', { filter, isLive: isLiveRef.current });

  // Reset logs when server-side filter changes so stale data from prior filter doesn't linger
  useEffect(() => {
    setLogs([]);
    setNewCount(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filter.level, filter.source, filter.task_id, filter.session_id]);

  const buildUrl = useCallback(
    () => {
      const params = new URLSearchParams();
      if (filter.level) params.set('level', filter.level);
      if (filter.source) params.set('source', filter.source);
      if (filter.task_id) params.set('task_id', filter.task_id);
      if (filter.session_id) params.set('session_id', filter.session_id);
      // Fetch logs from the last 1 hour — skip when filtering by task/session
      // so older logs from a specific task are still visible
      if (!filter.task_id && !filter.session_id) {
        const since = new Date(Date.now() - 60 * 60 * 1000).toISOString();
        params.set('since', since);
      }
      params.set('limit', '500');
      const url = `/api/logs?${params.toString()}`;
      logger.debug('useLogs: built URL', { url, filter });
      return url;
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [filter.level, filter.source, filter.task_id, filter.session_id]
  );

  const { isLoading } = useQuery({
    queryKey: ['logs', filter.level, filter.source, filter.task_id, filter.session_id],
    queryFn: async () => {
      const url = buildUrl();
      logger.debug('useLogs: fetching logs', { url });
      const fresh = await apiClient<LogEntry[]>(url);
      logger.debug('useLogs: fetch complete', { count: fresh.length });

      setLogs((prev) => {
        // Deduplicate by id, keep newest first
        const existingIds = new Set(prev.map((l) => l.id));
        const newEntries = fresh.filter((l) => !existingIds.has(l.id));

        if (newEntries.length === 0) {
          logger.debug('useLogs: no new entries', { prevCount: prev.length });
          return prev;
        }

        logger.debug('useLogs: adding new entries', { newCount: newEntries.length, prevCount: prev.length, isLive: isLiveRef.current });

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
    logger.debug('useLogs: loadNewLogs called');
    setLogs((prev) => prev); // trigger a re-render after setting live
    setNewCount(0);
  }, []);

  return { logs, isLoading, newCount, loadNewLogs, isLiveRef };
}
