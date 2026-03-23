import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';

describe('Logger', () => {
  // Don't use fake timers - they conflict with logger's setInterval

  it('buffers log entries without immediately flushing', async () => {
    let flushCount = 0;
    server.use(
      http.post('/api/logs', () => {
        flushCount++;
        return HttpResponse.json({}, { status: 201 });
      })
    );

    vi.resetModules();
    const { logger } = await import('./logger');
    logger.info('test message 1');
    logger.info('test message 2');

    // Give a small amount of time to ensure no immediate flush
    await new Promise((r) => setTimeout(r, 10));
    // No flush yet (timer hasn't fired after 10ms)
    expect(flushCount).toBe(0);
    logger.destroy();
  });

  it('flushes when flush() is called explicitly', async () => {
    const flushed: unknown[] = [];
    server.use(
      http.post('/api/logs', async ({ request }) => {
        flushed.push(await request.json());
        return HttpResponse.json({}, { status: 201 });
      })
    );

    vi.resetModules();
    const { logger } = await import('./logger');
    logger.info('explicit flush test');
    await logger.flush();

    expect(flushed.length).toBeGreaterThan(0);
    logger.destroy();
  });

  it('withContext creates child logger with merged context', async () => {
    const entries: unknown[] = [];
    server.use(
      http.post('/api/logs', async ({ request }) => {
        entries.push(await request.json());
        return HttpResponse.json({}, { status: 201 });
      })
    );

    vi.resetModules();
    const { logger } = await import('./logger');
    const child = logger.withContext({ task_id: 'task-123', target: 'TestComponent' });
    child.info('child message');

    await logger.flush();

    const found = entries.find(
      (e: unknown) => (e as Record<string, unknown>)?.message === 'child message'
    ) as Record<string, unknown> | undefined;
    expect(found?.task_id).toBe('task-123');
    expect(found?.target).toBe('TestComponent');
    logger.destroy();
  });

  it('deduplicates identical consecutive messages within 1 second', async () => {
    const entries: unknown[] = [];
    server.use(
      http.post('/api/logs', async ({ request }) => {
        entries.push(await request.json());
        return HttpResponse.json({}, { status: 201 });
      })
    );

    vi.resetModules();
    const { logger } = await import('./logger');
    logger.info('duplicate');
    logger.info('duplicate'); // Should be dropped
    logger.info('duplicate'); // Should be dropped

    await logger.flush();
    const dupes = entries.filter(
      (e: unknown) => (e as Record<string, unknown>)?.message === 'duplicate'
    );
    expect(dupes.length).toBe(1);
    logger.destroy();
  });

  it('never throws even if backend is down', async () => {
    server.use(
      http.post('/api/logs', () => {
        return HttpResponse.error();
      })
    );

    vi.resetModules();
    const { logger } = await import('./logger');
    logger.error('this will fail to send');
    await expect(logger.flush()).resolves.not.toThrow();
    logger.destroy();
  });
});
