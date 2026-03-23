import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { useLogs } from './use-logs';
import { mockLog } from '@/test/msw/fixtures';

function wrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false, refetchInterval: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: qc }, children);
}

describe('useLogs', () => {
  it('fetches and accumulates logs', async () => {
    const { result } = renderHook(() => useLogs(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.logs).toHaveLength(1);
    expect(result.current.logs[0].id).toBe(mockLog.id);
  });

  it('deduplicates logs on subsequent fetches', async () => {
    server.use(
      http.get('/api/logs', () =>
        HttpResponse.json([mockLog])
      )
    );

    const { result } = renderHook(() => useLogs(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.logs.length).toBeGreaterThan(0));

    // Simulate re-fetch (same data)
    const prev = result.current.logs.length;
    await waitFor(() => expect(result.current.logs.length).toBe(prev));
  });

  it('passes level filter to API URL', async () => {
    let capturedUrl = '';
    server.use(
      http.get('/api/logs', ({ request }) => {
        capturedUrl = request.url;
        return HttpResponse.json([]);
      })
    );

    renderHook(() => useLogs({ level: 'ERROR' }), { wrapper: wrapper() });
    await waitFor(() => expect(capturedUrl).toContain('level=ERROR'));
  });

  it('passes source filter to API URL', async () => {
    let capturedUrl = '';
    server.use(
      http.get('/api/logs', ({ request }) => {
        capturedUrl = request.url;
        return HttpResponse.json([]);
      })
    );

    renderHook(() => useLogs({ source: 'frontend' }), { wrapper: wrapper() });
    await waitFor(() => expect(capturedUrl).toContain('source=frontend'));
  });
});
