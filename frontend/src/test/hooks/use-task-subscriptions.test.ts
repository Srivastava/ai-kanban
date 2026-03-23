import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { useTaskSubscriptions } from '@/hooks/use-task-subscriptions';

// ──────────────────────────────────────────────
// Helpers / mocks
// ──────────────────────────────────────────────

/**
 * Build a minimal mock for `useWebSocket` so we can control `status` and
 * manually fire `subscribe` callbacks without a real WebSocket.
 */
function createMockWebSocket(status: 'connecting' | 'connected' | 'disconnected' = 'connected') {
  // Map of eventType -> Set<callback>
  const listeners = new Map<string, Set<(data: unknown) => void>>();

  const subscribe = vi.fn((eventType: string, callback: (data: unknown) => void) => {
    const set = listeners.get(eventType) ?? new Set();
    set.add(callback);
    listeners.set(eventType, set);

    // Return unsubscribe function
    return () => {
      const s = listeners.get(eventType);
      if (s) {
        s.delete(callback);
        if (s.size === 0) listeners.delete(eventType);
      }
    };
  });

  function emit(eventType: string, data: unknown = {}) {
    listeners.get(eventType)?.forEach((cb) => cb(data));
  }

  return { status, subscribe, emit, listeners };
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

describe('useTaskSubscriptions', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it('subscribes to task_created and task_deleted when connected', async () => {
    const mockWs = createMockWebSocket('connected');
    vi.doMock('@/contexts/websocket-context', () => ({
      useWebSocket: () => ({ subscribe: mockWs.subscribe, status: mockWs.status }),
    }));

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    vi.doMock('@tanstack/react-query', async (importOriginal) => {
      const original = await importOriginal<typeof import('@tanstack/react-query')>();
      return {
        ...original,
        useQueryClient: () => queryClient,
      };
    });

    const { useTaskSubscriptions: hook } = await import('@/hooks/use-task-subscriptions');
    renderHook(() => hook());

    expect(mockWs.subscribe).toHaveBeenCalledWith('task_created', expect.any(Function));
    expect(mockWs.subscribe).toHaveBeenCalledWith('task_deleted', expect.any(Function));
  });

  it('does NOT subscribe when status is not connected', async () => {
    const mockWs = createMockWebSocket('connecting');
    vi.doMock('@/contexts/websocket-context', () => ({
      useWebSocket: () => ({ subscribe: mockWs.subscribe, status: mockWs.status }),
    }));

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    vi.doMock('@tanstack/react-query', async (importOriginal) => {
      const original = await importOriginal<typeof import('@tanstack/react-query')>();
      return { ...original, useQueryClient: () => queryClient };
    });

    const { useTaskSubscriptions: hook } = await import('@/hooks/use-task-subscriptions');
    renderHook(() => hook());

    expect(mockWs.subscribe).not.toHaveBeenCalled();
  });

  it('invalidates ["tasks"] query when task_created fires', async () => {
    const mockWs = createMockWebSocket('connected');
    vi.doMock('@/contexts/websocket-context', () => ({
      useWebSocket: () => ({ subscribe: mockWs.subscribe, status: mockWs.status }),
    }));

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries');

    vi.doMock('@tanstack/react-query', async (importOriginal) => {
      const original = await importOriginal<typeof import('@tanstack/react-query')>();
      return { ...original, useQueryClient: () => queryClient };
    });

    const { useTaskSubscriptions: hook } = await import('@/hooks/use-task-subscriptions');
    renderHook(() => hook());

    act(() => {
      mockWs.emit('task_created', { type: 'task_created', task_id: 1 });
    });

    expect(invalidate).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: ['tasks'] })
    );
  });

  it('invalidates ["tasks"] query when task_deleted fires', async () => {
    const mockWs = createMockWebSocket('connected');
    vi.doMock('@/contexts/websocket-context', () => ({
      useWebSocket: () => ({ subscribe: mockWs.subscribe, status: mockWs.status }),
    }));

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries');

    vi.doMock('@tanstack/react-query', async (importOriginal) => {
      const original = await importOriginal<typeof import('@tanstack/react-query')>();
      return { ...original, useQueryClient: () => queryClient };
    });

    const { useTaskSubscriptions: hook } = await import('@/hooks/use-task-subscriptions');
    renderHook(() => hook());

    act(() => {
      mockWs.emit('task_deleted', { type: 'task_deleted', task_id: 5 });
    });

    expect(invalidate).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: ['tasks'] })
    );
  });

  it('unsubscribes on unmount', async () => {
    const mockWs = createMockWebSocket('connected');
    vi.doMock('@/contexts/websocket-context', () => ({
      useWebSocket: () => ({ subscribe: mockWs.subscribe, status: mockWs.status }),
    }));

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    vi.doMock('@tanstack/react-query', async (importOriginal) => {
      const original = await importOriginal<typeof import('@tanstack/react-query')>();
      return { ...original, useQueryClient: () => queryClient };
    });

    const { useTaskSubscriptions: hook } = await import('@/hooks/use-task-subscriptions');
    const { unmount } = renderHook(() => hook());

    expect(mockWs.listeners.has('task_created')).toBe(true);
    expect(mockWs.listeners.has('task_deleted')).toBe(true);

    unmount();

    // After unmount the unsubscribe functions should have been called,
    // leaving empty sets which get deleted from the map.
    expect(mockWs.listeners.has('task_created')).toBe(false);
    expect(mockWs.listeners.has('task_deleted')).toBe(false);
  });
});
