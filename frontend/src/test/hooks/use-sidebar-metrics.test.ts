import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useSidebarMetrics } from '@/hooks/use-sidebar-metrics';
import { mockOverview } from '@/test/msw/fixtures';

function wrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('useSidebarMetrics', () => {
  it('returns null when no data', () => {
    server.use(
      http.get('/api/analytics/overview', () =>
        HttpResponse.json(null, { status: 500 })
      )
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    expect(result.current).toBeNull();
  });

  it('returns metrics array when data is available', async () => {
    server.use(
      http.get('/api/analytics/overview', () => HttpResponse.json(mockOverview))
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current).not.toBeNull());
    expect(Array.isArray(result.current)).toBe(true);
    expect(result.current!.length).toBe(6);
  });

  it('includes Total Cost label', async () => {
    server.use(
      http.get('/api/analytics/overview', () => HttpResponse.json(mockOverview))
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current).not.toBeNull());
    const labels = result.current!.map((m) => m.label);
    expect(labels).toContain('Total Cost');
  });

  it('includes Sessions label', async () => {
    server.use(
      http.get('/api/analytics/overview', () => HttpResponse.json(mockOverview))
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current).not.toBeNull());
    const labels = result.current!.map((m) => m.label);
    expect(labels).toContain('Sessions');
  });

  it('formats cost correctly', async () => {
    server.use(
      http.get('/api/analytics/overview', () => HttpResponse.json(mockOverview))
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current).not.toBeNull());
    const costItem = result.current!.find((m) => m.label === 'Total Cost');
    expect(costItem?.value).toBe(`$${mockOverview.estimated_cost_usd.toFixed(2)}`);
  });

  it('shows dash for avg when no sessions', async () => {
    server.use(
      http.get('/api/analytics/overview', () =>
        HttpResponse.json({ ...mockOverview, total_sessions: 0 })
      )
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current).not.toBeNull());
    const avgItem = result.current!.find((m) => m.label === 'Avg / Session');
    expect(avgItem?.value).toBe('—');
  });

  it('formats large token counts with K suffix', async () => {
    server.use(
      http.get('/api/analytics/overview', () =>
        HttpResponse.json({ ...mockOverview, total_input_tokens: 50000, total_output_tokens: 50000, total_cache_creation_tokens: 0, total_cache_read_tokens: 0 })
      )
    );
    const { result } = renderHook(() => useSidebarMetrics(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current).not.toBeNull());
    const tokensItem = result.current!.find((m) => m.label === 'Tokens');
    expect(tokensItem?.value).toContain('K');
  });
});
