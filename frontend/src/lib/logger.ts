const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';
const FLUSH_INTERVAL_MS = 10_000;
const MAX_BUFFER_SIZE = 20;

interface LogEntry {
  level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
  message: string;
  source: 'frontend';
  target?: string;
  task_id?: string;
  session_id?: string;
  metadata?: Record<string, unknown>;
}

interface LogContext {
  task_id?: string;
  session_id?: string;
  target?: string;
}

class Logger {
  private buffer: LogEntry[] = [];
  private flushTimer: ReturnType<typeof setInterval> | null = null;
  private context: LogContext = {};
  private lastMessage = '';
  private lastMessageTime = 0;

  constructor() {
    if (typeof window !== 'undefined') {
      this.flushTimer = setInterval(() => this.flush(), FLUSH_INTERVAL_MS);
      window.addEventListener('beforeunload', () => this.flushSync());
    }
  }

  setContext(ctx: LogContext) {
    this.context = { ...this.context, ...ctx };
  }

  withContext(ctx: LogContext): ContextLogger {
    return new ContextLogger(this, { ...this.context, ...ctx });
  }

  debug(message: string, metadata?: Record<string, unknown>) {
    this.log('DEBUG', message, metadata);
  }

  info(message: string, metadata?: Record<string, unknown>) {
    this.log('INFO', message, metadata);
  }

  warn(message: string, metadata?: Record<string, unknown>) {
    this.log('WARN', message, metadata);
  }

  error(message: string, metadata?: Record<string, unknown>) {
    this.log('ERROR', message, metadata);
  }

  log(
    level: LogEntry['level'],
    message: string,
    metadata?: Record<string, unknown>,
    ctx?: LogContext
  ) {
    const now = Date.now();
    if (message === this.lastMessage && now - this.lastMessageTime < 1000) {
      return;
    }
    this.lastMessage = message;
    this.lastMessageTime = now;

    const merged = { ...this.context, ...ctx };
    const entry: LogEntry = {
      level,
      message,
      source: 'frontend',
      target: merged.target,
      task_id: merged.task_id,
      session_id: merged.session_id,
      metadata,
    };

    this.buffer.push(entry);

    if (this.buffer.length >= MAX_BUFFER_SIZE) {
      this.flush();
    }
  }

  async flush() {
    if (this.buffer.length === 0) return;

    const entries = this.buffer.splice(0, this.buffer.length);
    try {
      await Promise.allSettled(
        entries.map((entry) =>
          fetch(`${API_BASE}/api/logs`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(entry),
          })
        )
      );
    } catch {
      // Silent drop
    }
  }

  private flushSync() {
    if (this.buffer.length === 0) return;
    const entries = this.buffer.splice(0, this.buffer.length);
    for (const entry of entries) {
      try {
        navigator.sendBeacon(
          `${API_BASE}/api/logs`,
          new Blob([JSON.stringify(entry)], { type: 'application/json' })
        );
      } catch {
        // Silent drop
      }
    }
  }

  destroy() {
    if (this.flushTimer) {
      clearInterval(this.flushTimer);
    }
  }
}

class ContextLogger {
  constructor(
    private parent: Logger,
    private ctx: LogContext
  ) {}

  debug(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('DEBUG', message, metadata, this.ctx);
  }

  info(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('INFO', message, metadata, this.ctx);
  }

  warn(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('WARN', message, metadata, this.ctx);
  }

  error(message: string, metadata?: Record<string, unknown>) {
    this.parent.log('ERROR', message, metadata, this.ctx);
  }
}

export const logger = new Logger();
export type { LogContext, ContextLogger };
