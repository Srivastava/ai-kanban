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

const WS_URL =
  typeof window !== 'undefined'
    ? `ws://${window.location.hostname}:3001/ws`
    : 'ws://localhost:3001/ws';

export function WebSocketProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const queryClientRef = useRef(queryClient);
  queryClientRef.current = queryClient;

  const [ws, setWs] = useState<WebSocket | null>(null);
  const [status, setStatus] = useState<WebSocketStatus>('connecting');
  const [listeners, setListeners] = useState<Map<string, Set<(data: unknown) => void>>>(new Map());

  const connect = useCallback(() => {
    const socket = new WebSocket(WS_URL);

    socket.onopen = () => {
      logger.info('WebSocket connected', { url: WS_URL });
      setStatus('connected');
      setWs(socket);
    };

    socket.onclose = () => {
      logger.warn('WebSocket disconnected, reconnecting in 3s');
      setStatus('disconnected');
      setWs(null);
      // Reconnect after 3 seconds
      setTimeout(connect, 3000);
    };

    socket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);

        // Handle task_updated: sync query cache so boards update in real-time
        if (message.type === 'task_updated' && message.task) {
          const task = message.task as Task;
          queryClientRef.current.setQueryData(['tasks', task.id], task);
          queryClientRef.current.invalidateQueries({ queryKey: ['tasks'] });
        }

        const callbacks = listeners.get(message.type);
        if (callbacks) {
          callbacks.forEach((cb) => cb(message));
        }
        // Also call 'any' listeners
        const anyCallbacks = listeners.get('*');
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
  }, [listeners]);

  useEffect(() => {
    connect();
    return () => {
      ws?.close();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const subscribe = useCallback((eventType: string, callback: (data: unknown) => void) => {
    setListeners((prev) => {
      const next = new Map(prev);
      const callbacks = next.get(eventType) || new Set();
      callbacks.add(callback);
      next.set(eventType, callbacks);
      return next;
    });

    return () => {
      setListeners((prev) => {
        const next = new Map(prev);
        const callbacks = next.get(eventType);
        if (callbacks) {
          callbacks.delete(callback);
          if (callbacks.size === 0) {
            next.delete(eventType);
          }
        }
        return next;
      });
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
