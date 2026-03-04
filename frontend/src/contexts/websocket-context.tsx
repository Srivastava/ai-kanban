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
      ws?.close();
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
