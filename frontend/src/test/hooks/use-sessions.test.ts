import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useSession, useAllSessions, useStopSession } from '@/hooks/use-sessions';

function wrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

const mockSession = {
  id: 'session-1',
  task_id: 'task-123',
  status: 'completed',
  stage: 'coding',
  created_at: '2026-03-01T10:00:00Z',
};

describe('useSession', () => {
  it('is disabled when sessionId is null', () => {
    const { result } = renderHook(() => useSession(null), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });

  it('is disabled when sessionId is undefined', () => {
    const { result } = renderHook(() => useSession(undefined), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });

  it('returns session data for valid ID', async () => {
    server.use(
      http.get('/api/sessions/:id', () => HttpResponse.json(mockSession))
    );
    const { result } = renderHook(() => useSession('session-1'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.id).toBe('session-1');
  });

  it('returns null for 404 session (not found)', async () => {
    server.use(
      http.get('/api/sessions/:id', () =>
        HttpResponse.json({ error: 'Not found' }, { status: 404 })
      )
    );
    const { result } = renderHook(() => useSession('missing-id'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toBeNull();
  });
});

describe('useAllSessions', () => {
  it('fetches all sessions', async () => {
    server.use(
      http.get('/api/sessions/all', () => HttpResponse.json([mockSession]))
    );
    const { result } = renderHook(() => useAllSessions(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(1);
  });

  it('fetches with status filter', async () => {
    server.use(
      http.get('/api/sessions/all', () => HttpResponse.json([mockSession]))
    );
    const { result } = renderHook(() => useAllSessions(['completed']), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});

describe('useStopSession', () => {
  it('has a mutate function', () => {
    const { result } = renderHook(() => useStopSession('task-123'), { wrapper: wrapper() });
    expect(typeof result.current.mutate).toBe('function');
  });

  it('is idle initially', () => {
    const { result } = renderHook(() => useStopSession('task-123'), { wrapper: wrapper() });
    expect(result.current.isIdle).toBe(true);
  });
});
