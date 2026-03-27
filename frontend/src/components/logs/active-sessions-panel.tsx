'use client';

import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { useTokensBySession } from '@/hooks/use-analytics';
import type { SessionTokens } from '@/types/analytics';

import { formatTokens as fmtTokens } from '@/lib/format';

function fmtTime(iso: string | null) {
  if (!iso) return '—';
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(iso));
}

function shortId(id: string) {
  return id.slice(0, 8);
}

interface Props {
  onSessionClick: (sessionId: string) => void;
  activeSessionId?: string;
}

export function ActiveSessionsPanel({ onSessionClick, activeSessionId }: Props) {
  const [open, setOpen] = useState(true);
  const { data: sessions = [], isLoading } = useTokensBySession();

  const sorted = [...sessions]
    .sort((a, b) => {
      if (!a.started_at) return 1;
      if (!b.started_at) return -1;
      return b.started_at.localeCompare(a.started_at);
    })
    .slice(0, 10);

  if (!isLoading && sorted.length === 0) return null;

  return (
    <div className="rounded-xl border border-border bg-card overflow-hidden">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-4 py-2.5 text-xs font-semibold text-muted-foreground uppercase tracking-wide hover:bg-muted/40 transition-colors"
      >
        <span className="w-2 h-2 rounded-full bg-primary/60" />
        Recent Sessions
        <span className="text-muted-foreground/60 font-normal normal-case ml-1">
          {sorted.length} session{sorted.length !== 1 ? 's' : ''}
        </span>
        <ChevronDown className={`h-3.5 w-3.5 ml-auto transition-transform duration-300 ease-out motion-reduce:transition-none ${open ? '' : '-rotate-90'}`} />
      </button>

      <div className="collapse-grid" style={{ gridTemplateRows: open ? '1fr' : '0fr' }}>
        <div className="overflow-hidden">
          <div className="border-t border-border">
            {isLoading ? (
              <div className="px-4 py-3 space-y-2">
                {[1, 2, 3].map((i) => <div key={i} className="h-6 bg-muted rounded animate-shimmer" />)}
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-muted-foreground border-b border-border/50 bg-muted/20">
                      <th className="text-left px-4 py-1.5 font-medium">Session ID</th>
                      <th className="text-left px-3 py-1.5 font-medium">Task</th>
                      <th className="text-left px-3 py-1.5 font-medium">Started</th>
                      <th className="text-right px-4 py-1.5 font-medium">in / cached / out</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/40">
                    {sorted.map((s) => (
                      <SessionRow
                        key={s.session_id}
                        session={s}
                        isActive={activeSessionId === s.session_id}
                        onSessionClick={onSessionClick}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function SessionRow({
  session,
  isActive,
  onSessionClick,
}: {
  session: SessionTokens;
  isActive: boolean;
  onSessionClick: (id: string) => void;
}) {
  return (
    <tr className={`transition-colors ${isActive ? 'bg-primary/10' : 'hover:bg-muted/30'}`}>
      <td className="px-4 py-1.5">
        <button
          className="font-mono text-primary hover:underline"
          onClick={() => onSessionClick(session.session_id)}
          title={session.session_id}
        >
          {shortId(session.session_id)}…
        </button>
      </td>
      <td className="px-3 py-1.5 text-muted-foreground truncate max-w-[200px]" title={session.task_title}>
        {session.task_title}
      </td>
      <td className="px-3 py-1.5 text-muted-foreground whitespace-nowrap">
        {fmtTime(session.started_at)}
      </td>
      <td className="px-4 py-1.5 text-right font-mono text-[11px] text-muted-foreground">
        <span>{fmtTokens(session.input_tokens)}↑</span>
        {' '}
        <span className="text-amber-500/80">{fmtTokens((session.cache_creation_tokens ?? 0) + (session.cache_read_tokens ?? 0))}⚡</span>
        {' '}
        <span>{fmtTokens(session.output_tokens)}↓</span>
      </td>
    </tr>
  );
}
