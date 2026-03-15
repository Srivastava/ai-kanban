'use client';

import { useEffect, useState } from 'react';

interface Props {
  label: string;
  used: number;
  limit: number;
  resetAt: string | null; // ISO-8601 timestamp
  /** If true, shows absolute date/time instead of a live countdown */
  showDate?: boolean;
}

function formatCountdown(resetAt: string): string {
  const diff = new Date(resetAt).getTime() - Date.now();
  if (diff <= 0) return 'now';
  const h = Math.floor(diff / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  const s = Math.floor((diff % 60_000) / 1_000);
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function formatResetDate(resetAt: string): string {
  const d = new Date(resetAt);
  const parts = d.toLocaleString('en-US', {
    timeZone: 'America/Los_Angeles',
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: true,
  });
  return `${parts} PST`;
}

function formatTokens(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function RateLimitGauge({ label, used, limit, resetAt, showDate = false }: Props) {
  // Start null to avoid SSR/client Date.now() mismatch
  const [countdown, setCountdown] = useState<string | null>(null);
  const [resetLabel, setResetLabel] = useState<string | null>(null);

  useEffect(() => {
    if (!resetAt) return;
    if (showDate) {
      setResetLabel(formatResetDate(resetAt));
    } else {
      setCountdown(formatCountdown(resetAt));
      const id = setInterval(() => setCountdown(formatCountdown(resetAt)), 1_000);
      return () => clearInterval(id);
    }
  }, [resetAt, showDate]);

  const pct = limit > 0 ? Math.min((used / limit) * 100, 100) : 0;
  const color = pct < 60 ? 'bg-emerald-500' : pct < 85 ? 'bg-amber-500' : 'bg-red-500';
  const textColor = pct < 60 ? 'text-emerald-400' : pct < 85 ? 'text-amber-400' : 'text-red-400';

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-foreground">{label}</span>
        <span className={textColor}>
          {formatTokens(used)} / {formatTokens(limit)} ({pct.toFixed(1)}%)
        </span>
      </div>
      <div className="h-2 w-full rounded-full bg-muted overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      {resetAt && (
        <p className="text-xs text-muted-foreground">
          {showDate
            ? <>Resets <span className="font-medium text-foreground">{resetLabel ?? '…'}</span></>
            : <>Resets in <span className="font-mono">{countdown ?? '…'}</span></>
          }
        </p>
      )}
    </div>
  );
}
