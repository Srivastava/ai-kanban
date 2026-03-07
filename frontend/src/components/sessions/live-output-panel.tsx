'use client';

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import { logger } from '@/lib/logger';
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
  const [rateLimitResetAt, setRateLimitResetAt] = useState<Date | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const { subscribe, send, status: wsStatus } = useWebSocket();
  const isConnected = wsStatus === 'connected';

  // Build a context-aware logger, updated when claudeSessionId changes
  const logRef = useRef(logger.withContext({ session_id: sessionId }));
  useEffect(() => {
    logRef.current = logger.withContext({
      session_id: sessionId,
      claude_session_id: claudeSessionId ?? undefined,
    });
  }, [sessionId, claudeSessionId]);

  // Reset lines, heartbeat, and claudeSessionId when session changes
  useEffect(() => {
    setLines([]);
    setHeartbeat(null);
    setClaudeSessionId(initialClaudeSessionId ?? null);
    logRef.current = logger.withContext({ session_id: sessionId });
    logRef.current.info('LiveOutputPanel: session changed', { session_id: sessionId });
  }, [sessionId, initialClaudeSessionId]);

  // Subscribe to this session's output and heartbeat
  useEffect(() => {
    if (!sessionId || !isConnected) return;

    logRef.current.info('LiveOutputPanel: subscribing to session output', { session_id: sessionId });
    send({ type: 'subscribe_session', session_id: sessionId });

    let outputCount = 0;
    const unsubOutput = subscribe('session_output', (data: unknown) => {
      const msg = data as { session_id: string; output: string; is_error: boolean };
      if (msg.session_id !== sessionId) return;
      outputCount++;
      if (outputCount === 1 || outputCount % 50 === 0) {
        logRef.current.debug('LiveOutputPanel: output received', {
          session_id: sessionId,
          is_error: msg.is_error,
          total_lines: outputCount,
        });
      }
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
      logRef.current.info('LiveOutputPanel: claude_session_id assigned', {
        session_id: sessionId,
        claude_session_id: msg.claude_session_id,
      });
      setClaudeSessionId(msg.claude_session_id);
    });

    return () => {
      logRef.current.debug('LiveOutputPanel: unsubscribing from session', {
        session_id: sessionId,
        total_lines_received: outputCount,
      });
      unsubOutput();
      unsubHeartbeat();
      unsubSessionId();
    };
  }, [sessionId, isConnected, subscribe, send]);

  // Subscribe to rate_limited events for this session
  useEffect(() => {
    if (!sessionId) return;
    return subscribe('rate_limited', (data: unknown) => {
      const event = data as import('@/types/session').RateLimitedEvent;
      if (event.session_id === sessionId) {
        logRef.current.warn('LiveOutputPanel: rate limited', {
          session_id: sessionId,
          reset_at: event.reset_at,
        });
        setRateLimitResetAt(new Date(event.reset_at));
      }
    });
  }, [sessionId, subscribe]);

  // Clear rate limit banner when session reaches a terminal state
  useEffect(() => {
    if (status === 'completed' || status === 'failed' || status === 'stopped') {
      setRateLimitResetAt(null);
    }
  }, [status]);

  // Live countdown ticker — ticks every second and auto-clears 10s after reset time
  useEffect(() => {
    if (!rateLimitResetAt) return;
    const interval = setInterval(() => {
      // Force re-render by creating a new Date object (same time)
      setRateLimitResetAt(prev => prev ? new Date(prev.getTime()) : null);
      // Auto-clear 10s after the reset time passes
      if (Date.now() > rateLimitResetAt.getTime() + 10_000) {
        setRateLimitResetAt(null);
      }
    }, 1000);
    return () => clearInterval(interval);
  }, [rateLimitResetAt]);

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

  const rateLimitCountdown = rateLimitResetAt
    ? (() => {
        const secsLeft = Math.max(0, Math.round((rateLimitResetAt.getTime() - Date.now()) / 1000));
        if (secsLeft === 0) return 'Resuming now…';
        const h = Math.floor(secsLeft / 3600);
        const m = Math.floor((secsLeft % 3600) / 60);
        const s = secsLeft % 60;
        return h > 0
          ? `Rate limited — resuming in ${h}h ${m}m`
          : `Rate limited — resuming in ${m}m ${s}s`;
      })()
    : null;
  const secsSinceHeartbeat = heartbeat
    ? Math.floor((Date.now() - heartbeat.receivedAt) / 1000)
    : 999;
  const isWaiting = isRunning && heartbeat !== null && secsSinceHeartbeat > 8;
  const hasActiveHeartbeat = isRunning && heartbeat !== null && !isWaiting;

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {rateLimitResetAt && (
        <div className="px-4 py-2 bg-amber-50 border-b border-amber-200 text-amber-800 text-sm flex items-center gap-2">
          <span>⏳</span>
          <span>{rateLimitCountdown}</span>
          <span className="text-amber-600 text-xs ml-auto">
            Auto-resumes at {rateLimitResetAt.toLocaleTimeString()}
          </span>
        </div>
      )}
      <div className="flex items-center justify-between px-3 py-2 bg-muted/40 border-b border-border">
        <div className="flex flex-col gap-0.5">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            Session Output
          </span>
          {claudeSessionId && (
            <span className="text-[10px] font-mono text-muted-foreground/60">
              Claude session: {claudeSessionId}
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
