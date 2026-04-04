'use client';

import { createContext, useContext, useEffect, useState, useCallback, useRef, ReactNode } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { logger } from '@/lib/logger';
import type { Task } from '@/types/task';

type WebSocketStatus = 'connecting' | 'connected' | 'disconnected';

interface WebSocketContextType {
  ws: WebSocket | null;
  status: WebSocketStatus;
  subscribe: (eventType: string, callback: (data: unknown) => void) => () => void;
  send: (data: object) => void;
}

const WebSocketContext = createContext<WebSocketContextType | null>(null);

// Derive WebSocket URL from the page's own origin so it works behind any reverse proxy.
// wss: when the page is HTTPS, ws: otherwise.
const WS_URL =
  typeof window !== 'undefined'
    ? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`
    : 'ws://localhost:3001/ws';

export function WebSocketProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const queryClientRef = useRef(queryClient);
  queryClientRef.current = queryClient;

  const [ws, setWs] = useState<WebSocket | null>(null);
  const [status, setStatus] = useState<WebSocketStatus>('connecting');
  const listenersRef = useRef<Map<string, Set<(data: unknown) => void>>>(new Map());
  // Hold the live socket in a ref so cleanup always sees the current instance,
  // avoiding the stale-closure bug where useEffect cleanup captures null.
  const wsRef = useRef<WebSocket | null>(null);
  // Hold the pending reconnect timer so it can be cancelled on unmount.
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    const socket = new WebSocket(WS_URL);
    wsRef.current = socket;

    socket.onopen = () => {
      logger.info('WebSocket connected', { url: WS_URL });
      setStatus('connected');
      setWs(socket);
    };

    socket.onclose = () => {
      logger.warn('WebSocket disconnected, reconnecting in 3s');
      setStatus('disconnected');
      setWs(null);
      wsRef.current = null;
      // Reconnect after 3 seconds
      reconnectTimerRef.current = setTimeout(connect, 3000);
    };

    socket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);

        // Log meaningful session/task events with context
        if (message.type === 'session_output' || message.type === 'session_heartbeat') {
          // high-frequency — skip logging
        } else if (message.session_id || message.task_id) {
          logger.withContext({
            session_id: message.session_id,
            task_id: message.task_id ?? message.task?.id,
            claude_session_id: message.claude_session_id,
          }).debug(`WS: ${message.type}`, {
            session_id: message.session_id,
            task_id: message.task_id ?? message.task?.id,
          });
        }

        // Handle task_updated: sync query cache so boards update in real-time
        if (message.type === 'task_updated' && message.task) {
          const task = message.task as Task;
          logger.withContext({ task_id: task.id }).info('WS: task updated', {
            task_id: task.id,
            stage: task.stage,
            title: task.title,
          });
          queryClientRef.current.setQueryData(['tasks', task.id], task);
          queryClientRef.current.invalidateQueries({ queryKey: ['tasks'] });
        }

        if (message.type === 'session_started') {
          logger.withContext({ task_id: message.task_id, session_id: message.session_id })
            .info('WS: session started', { task_id: message.task_id, session_id: message.session_id });
        }

        if (message.type === 'session_completed' || message.type === 'session_failed' || message.type === 'session_stopped') {
          logger.withContext({ task_id: message.task_id, session_id: message.session_id })
            .info(`WS: ${message.type}`, {
              task_id: message.task_id,
              session_id: message.session_id,
              error: message.error,
            });
        }

        const callbacks = listenersRef.current.get(message.type);
        if (callbacks) {
          callbacks.forEach((cb) => cb(message));
        }
        // Also call 'any' listeners
        const anyCallbacks = listenersRef.current.get('*');
        if (anyCallbacks) {
          anyCallbacks.forEach((cb) => cb(message));
        }
      } catch {
        logger.error('Failed to parse WebSocket message');
      }
    };

    socket.onerror = (error) => {
      logger.error('WebSocket error', { error: String(error) });
    };
  }, []);

  useEffect(() => {
    connect();
    return () => {
      // Cancel any pending reconnect timer to avoid reconnecting after unmount
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      // Close the live socket via ref — not via state, which is stale here
      wsRef.current?.close();
      wsRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const subscribe = useCallback((eventType: string, callback: (data: unknown) => void) => {
    const callbacks = listenersRef.current.get(eventType) || new Set();
    callbacks.add(callback);
    listenersRef.current.set(eventType, callbacks);

    return () => {
      const cbs = listenersRef.current.get(eventType);
      if (cbs) {
        cbs.delete(callback);
        if (cbs.size === 0) {
          listenersRef.current.delete(eventType);
        }
      }
    };
  }, []);

  const send = useCallback((data: object) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(data));
    }
  }, [ws]);

  return (
    <WebSocketContext.Provider value={{ ws, status, subscribe, send }}>
      {children}
    </WebSocketContext.Provider>
  );
}

export function useWebSocket() {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocket must be used within WebSocketProvider');
  }
  return context;
}
