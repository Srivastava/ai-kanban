'use client';

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import type { SessionStatus } from '@/types/session';

interface Props {
  sessionId: string;
  status: SessionStatus | null | undefined;
  initialClaudeSessionId?: string | null;
}

interface OutputLine {
  text: string;
  isError: boolean;
}

interface HeartbeatState {
  elapsedSecs: number;
  receivedAt: number; // Date.now() when we got it
}

export function LiveOutputPanel({ sessionId, status, initialClaudeSessionId }: Props) {
  const [lines, setLines] = useState<OutputLine[]>([]);
  const [heartbeat, setHeartbeat] = useState<HeartbeatState | null>(null);
  const [displayElapsed, setDisplayElapsed] = useState<number>(0);
  const [claudeSessionId, setClaudeSessionId] = useState<string | null>(initialClaudeSessionId ?? null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const { subscribe, send, status: wsStatus } = useWebSocket();
  const isConnected = wsStatus === 'connected';

  // Reset lines, heartbeat, and claudeSessionId when session changes
  useEffect(() => {
    setLines([]);
    setHeartbeat(null);
    setClaudeSessionId(initialClaudeSessionId ?? null);
  }, [sessionId, initialClaudeSessionId]);

  // Subscribe to this session's output and heartbeat
  useEffect(() => {
    if (!sessionId || !isConnected) return;

    send({ type: 'subscribe_session', session_id: sessionId });

    const unsubOutput = subscribe('session_output', (data: unknown) => {
      const msg = data as { session_id: string; output: string; is_error: boolean };
      if (msg.session_id !== sessionId) return;
      setLines((prev) => [
        ...prev.slice(-500),
        { text: msg.output, isError: msg.is_error },
      ]);
    });

    const unsubHeartbeat = subscribe('session_heartbeat', (data: unknown) => {
      const msg = data as { session_id: string; elapsed_secs: number };
      if (msg.session_id !== sessionId) return;
      setHeartbeat({ elapsedSecs: msg.elapsed_secs, receivedAt: Date.now() });
    });

    const unsubSessionId = subscribe('session_id_assigned', (data: unknown) => {
      const msg = data as { session_id: string; claude_session_id: string };
      if (msg.session_id !== sessionId) return;
      setClaudeSessionId(msg.claude_session_id);
    });

    return () => {
      unsubOutput();
      unsubHeartbeat();
      unsubSessionId();
    };
  }, [sessionId, isConnected, subscribe, send]);

  // Tick display elapsed every second while running
  useEffect(() => {
    const isRunning = status === 'running' || status === 'pending';
    if (!isRunning || !heartbeat) return;

    const update = () => {
      const secsSinceHeartbeat = Math.floor((Date.now() - heartbeat.receivedAt) / 1000);
      setDisplayElapsed(heartbeat.elapsedSecs + secsSinceHeartbeat);
    };

    update(); // set immediately
    const interval = setInterval(update, 1000);
    return () => clearInterval(interval);
  }, [heartbeat, status]);

  // Auto-scroll to bottom
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [lines]);

  if (!sessionId) return null;

  const isRunning = status === 'running' || status === 'pending';
  const secsSinceHeartbeat = heartbeat
    ? Math.floor((Date.now() - heartbeat.receivedAt) / 1000)
    : 999;
  const isWaiting = isRunning && heartbeat !== null && secsSinceHeartbeat > 8;
  const hasActiveHeartbeat = isRunning && heartbeat !== null && !isWaiting;

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-muted/40 border-b border-border">
        <div className="flex flex-col gap-0.5">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            Session Output
          </span>
          {claudeSessionId && (
            <span className="text-[10px] font-mono text-muted-foreground/60" title={claudeSessionId}>
              Claude session: {claudeSessionId.slice(0, 8)}…
            </span>
          )}
        </div>
        {hasActiveHeartbeat && (
          <span className="flex items-center gap-1.5 text-xs text-emerald-500">
            <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
            Running {displayElapsed}s
          </span>
        )}
        {isWaiting && (
          <span className="flex items-center gap-1.5 text-xs text-yellow-500">
            <span className="h-2 w-2 rounded-full bg-yellow-500 animate-pulse" />
            Waiting...
          </span>
        )}
        {isRunning && heartbeat === null && (
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
