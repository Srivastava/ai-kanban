'use client';

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import type { SessionStatus } from '@/types/session';

interface Props {
  sessionId: string;
  status: SessionStatus | null | undefined;
}

interface OutputLine {
  text: string;
  isError: boolean;
}

export function LiveOutputPanel({ sessionId, status }: Props) {
  const [lines, setLines] = useState<OutputLine[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);
  const { subscribe, send, status: wsStatus } = useWebSocket();
  const isConnected = wsStatus === 'connected';

  // Reset lines when session changes
  useEffect(() => {
    setLines([]);
  }, [sessionId]);

  // Subscribe to this session's output
  useEffect(() => {
    if (!sessionId || !isConnected) return;

    // Tell the server we want this session's output
    send({ type: 'subscribe_session', session_id: sessionId });

    const unsub = subscribe('session_output', (data: unknown) => {
      const msg = data as { session_id: string; output: string; is_error: boolean };
      if (msg.session_id !== sessionId) return;
      setLines((prev) => [
        ...prev.slice(-500),
        { text: msg.output, isError: msg.is_error },
      ]);
    });

    return unsub;
  }, [sessionId, isConnected, subscribe, send]);

  // Auto-scroll to bottom
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [lines]);

  if (!sessionId) return null;

  const isRunning = status === 'running' || status === 'pending';

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-muted/40 border-b border-border">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Session Output
        </span>
        {isRunning && (
          <span className="flex items-center gap-1.5 text-xs text-emerald-500">
            <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
            Live
          </span>
        )}
        {!isRunning && lines.length > 0 && (
          <span className="text-xs text-muted-foreground">Completed</span>
        )}
      </div>
      <div className="max-h-64 overflow-y-auto bg-black/90 p-3 font-mono text-xs">
        {lines.length === 0 ? (
          <p className="text-muted-foreground italic">
            {isRunning ? 'Waiting for output...' : 'No output captured.'}
          </p>
        ) : (
          lines.map((line, i) => (
            <div
              key={i}
              className={`whitespace-pre-wrap ${line.isError ? 'text-red-400' : 'text-green-300/90'}`}
            >
              {line.text}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
