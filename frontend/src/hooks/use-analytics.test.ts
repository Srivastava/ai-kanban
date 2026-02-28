import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import {
  useAnalyticsOverview,
  useDailyTokens,
  useTokensByTool,
  useSessionTimeline,
} from './use-analytics';
import { mockOverview } from '@/test/msw/fixtures';

function wrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: qc }, children);
}

describe('useAnalyticsOverview', () => {
  it('returns overview data', async () => {
    const { result } = renderHook(() => useAnalyticsOverview(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.total_sessions).toBe(mockOverview.total_sessions);
    expect(result.current.data?.estimated_cost_usd).toBe(mockOverview.estimated_cost_usd);
  });
});

describe('useDailyTokens', () => {
  it('returns array of daily data points', async () => {
    const { result } = renderHook(() => useDailyTokens(30), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
    expect(result.current.data!.length).toBeGreaterThan(0);
    expect(result.current.data![0]).toHaveProperty('date');
    expect(result.current.data![0]).toHaveProperty('input_tokens');
  });
});

describe('useTokensByTool', () => {
  it('returns tool breakdown', async () => {
    const { result } = renderHook(() => useTokensByTool(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data![0].tool_name).toBe('Read');
  });
});

describe('useSessionTimeline', () => {
  it('is disabled when sessionId is null', () => {
    const { result } = renderHook(() => useSessionTimeline(null), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });

  it('fetches when sessionId is provided', async () => {
    const { result } = renderHook(() => useSessionTimeline('sess-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});
