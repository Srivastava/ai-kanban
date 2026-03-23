import { describe, it, expect } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useFeatureFlags, useUpdateFeatureFlag } from '@/hooks/use-settings';
import type { FeatureFlag } from '@/hooks/use-settings';

function wrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

const mockFlag: FeatureFlag = {
  key: 'enable_analytics',
  enabled: true,
  updated_at: '2026-03-01T10:00:00Z',
};

const mockFlag2: FeatureFlag = {
  key: 'enable_comments',
  enabled: false,
  updated_at: '2026-03-01T10:00:00Z',
};

describe('useFeatureFlags', () => {
  it('returns feature flags from API', async () => {
    server.use(
      http.get('/api/settings/flags', () =>
        HttpResponse.json([mockFlag, mockFlag2])
      )
    );
    const { result } = renderHook(() => useFeatureFlags(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(2);
  });

  it('returns flags with correct shape', async () => {
    server.use(
      http.get('/api/settings/flags', () =>
        HttpResponse.json([mockFlag])
      )
    );
    const { result } = renderHook(() => useFeatureFlags(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    const flag = result.current.data![0];
    expect(flag).toHaveProperty('key');
    expect(flag).toHaveProperty('enabled');
    expect(flag).toHaveProperty('updated_at');
  });

  it('returns empty array when no flags configured', async () => {
    server.use(
      http.get('/api/settings/flags', () =>
        HttpResponse.json([])
      )
    );
    const { result } = renderHook(() => useFeatureFlags(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(0);
  });

  it('returns error state on API failure', async () => {
    server.use(
      http.get('/api/settings/flags', () =>
        HttpResponse.json({ error: 'Server error' }, { status: 500 })
      )
    );
    const { result } = renderHook(() => useFeatureFlags(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });

  it('reflects correct enabled state for each flag', async () => {
    server.use(
      http.get('/api/settings/flags', () =>
        HttpResponse.json([mockFlag, mockFlag2])
      )
    );
    const { result } = renderHook(() => useFeatureFlags(), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    const analyticsFlag = result.current.data!.find(f => f.key === 'enable_analytics');
    const commentsFlag = result.current.data!.find(f => f.key === 'enable_comments');
    expect(analyticsFlag?.enabled).toBe(true);
    expect(commentsFlag?.enabled).toBe(false);
  });
});

describe('useUpdateFeatureFlag', () => {
  it('has mutate function', () => {
    const { result } = renderHook(() => useUpdateFeatureFlag(), { wrapper: wrapper() });
    expect(typeof result.current.mutate).toBe('function');
  });

  it('is in idle state initially', () => {
    const { result } = renderHook(() => useUpdateFeatureFlag(), { wrapper: wrapper() });
    expect(result.current.isIdle).toBe(true);
  });

  it('updates a feature flag successfully', async () => {
    const updatedFlag = { ...mockFlag, enabled: false };
    server.use(
      http.patch('/api/settings/flags/:key', () =>
        HttpResponse.json(updatedFlag)
      )
    );
    const { result } = renderHook(() => useUpdateFeatureFlag(), { wrapper: wrapper() });
    act(() => {
      result.current.mutate({ key: 'enable_analytics', enabled: false });
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.enabled).toBe(false);
  });

  it('enters error state on API failure', async () => {
    server.use(
      http.patch('/api/settings/flags/:key', () =>
        HttpResponse.json({ error: 'Not found' }, { status: 404 })
      )
    );
    const { result } = renderHook(() => useUpdateFeatureFlag(), { wrapper: wrapper() });
    act(() => {
      result.current.mutate({ key: 'nonexistent', enabled: true });
    });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});
