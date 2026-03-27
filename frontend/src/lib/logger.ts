function getApiBase() {
  // Use relative URLs so the request always goes through whatever host served the page
  // (works both locally and behind a reverse proxy).
  if (typeof window !== 'undefined') return '';
  return 'http://localhost:3001';
}
const FLUSH_INTERVAL_MS = 5_000; // Reduced from 10s to 5s
const MAX_BUFFER_SIZE = 10; // Reduced from 20 to 10

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
  claude_session_id?: string;
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
      // Log that logger is initialized
      this.info('Logger initialized', { apiBase: getApiBase(), flushInterval: FLUSH_INTERVAL_MS });
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
    // Flush errors immediately
    this.flush();
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
    // Embed claude_session_id in metadata so it's searchable and stored
    const enrichedMetadata: Record<string, unknown> | undefined =
      merged.claude_session_id
        ? { claude_session_id: merged.claude_session_id, ...metadata }
        : metadata;
    const entry: LogEntry = {
      level,
      message,
      source: 'frontend',
      target: merged.target,
      task_id: merged.task_id,
      session_id: merged.session_id,
      metadata: enrichedMetadata,
    };

    // In production: only ERROR/WARN reach the console — DEBUG/INFO are silent.
    // In development: all levels are logged for visibility.
    const isProd = process.env.NODE_ENV === 'production';
    if (!isProd || level === 'ERROR' || level === 'WARN') {
      const consoleMethod = level === 'ERROR' ? 'error' : level === 'WARN' ? 'warn' : level === 'DEBUG' ? 'debug' : 'log';
      console[consoleMethod](`[${level}] [frontend] ${message}`, metadata || '');
    }

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
          fetch(`${getApiBase()}/api/logs`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(entry),
          })
        )
      );
    } catch (err) {
      console.error('[Logger] Flush failed:', err);
    }
  }

  private flushSync() {
    if (this.buffer.length === 0) return;
    const entries = this.buffer.splice(0, this.buffer.length);
    for (const entry of entries) {
      try {
        navigator.sendBeacon(
          `${getApiBase()}/api/logs`,
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
