'use client';

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

interface SessionOutputProps {
  sessionId: string;
}

export function SessionOutput({ sessionId }: SessionOutputProps) {
  const [lines, setLines] = useState<string[]>([]);
  const { subscribe, status } = useWebSocket();
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (status !== 'connected') return;

    const unsubscribe = subscribe('session_output', (data: unknown) => {
      const message = data as { session_id: string; output: string };
      if (message.session_id === sessionId) {
        setLines((prev) => [...prev, message.output]);
      }
    });

    return unsubscribe;
  }, [status, subscribe, sessionId]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines]);

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm">Session Output</CardTitle>
      </CardHeader>
      <CardContent>
        <div
          ref={scrollRef}
          className="bg-muted rounded-md p-3 h-64 overflow-y-auto font-mono text-xs"
        >
          {lines.length === 0 ? (
            <p className="text-muted-foreground">Waiting for output...</p>
          ) : (
            lines.map((line, i) => (
              <div key={i} className="whitespace-pre-wrap">
                {line}
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
