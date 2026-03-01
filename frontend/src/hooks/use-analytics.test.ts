import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import type { ReactNode } from 'react';
import {
  useAnalyticsOverview,
  useDailyTokens,
  useTokensByTool,
  useSessionTimeline,
  useCostByTask,
  useTokensByStage,
  useSessionSummary,
  useBurnRate,
} from './use-analytics';
import { mockOverview, mockCostByTask, mockTokensByStage, mockSessionSummary, mockBurnRate } from '@/test/msw/fixtures';

function wrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: qc }, children);
}

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  return ({ children }: { children: ReactNode }) =>
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

describe('useCostByTask', () => {
  it('returns cost data', async () => {
    const { result } = renderHook(() => useCostByTask(), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(mockCostByTask.length);
    expect(result.current.data![0].cost_usd).toBe(mockCostByTask[0].cost_usd);
  });
});

describe('useTokensByStage', () => {
  it('returns stage data', async () => {
    const { result } = renderHook(() => useTokensByStage(), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(mockTokensByStage.length);
  });
});

describe('useSessionSummary', () => {
  it('returns summary', async () => {
    const { result } = renderHook(() => useSessionSummary(), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data!.total_sessions).toBe(mockSessionSummary.total_sessions);
  });
});

describe('useBurnRate', () => {
  it('returns burn rate', async () => {
    const { result } = renderHook(() => useBurnRate(), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data!.tokens_per_minute).toBe(mockBurnRate.tokens_per_minute);
  });
});
