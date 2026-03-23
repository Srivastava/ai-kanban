/**
 * Tests for WebSocketProvider and useWebSocket hook.
 *
 * The MSW server (started in setup.ts) also intercepts WebSocket connections,
 * wrapping whatever class it finds in the global. To avoid conflicts we give
 * our MockWebSocket a minimal EventTarget surface so MSW's wrapper doesn't
 * throw, then drive behaviour through the on* handler properties that the
 * context itself sets.
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';

// ──────────────────────────────────────────────
// Hoisted mock – must run before any module imports
// ──────────────────────────────────────────────

const { MockWebSocketClass, getLastInstance, resetInstance } = vi.hoisted(() => {
  class MockWebSocket {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;

    readyState = 0; // CONNECTING
    url: string;

    onopen: ((e: Event) => void) | null = null;
    onclose: ((e: CloseEvent) => void) | null = null;
    onmessage: ((e: MessageEvent) => void) | null = null;
    onerror: ((e: Event) => void) | null = null;

    // Minimal EventTarget so MSW's wrapper doesn't crash
    private _listeners = new Map<string, Set<EventListenerOrEventListenerObject>>();

    constructor(url: string) {
      this.url = url;
      lastInstance = this as unknown as MockWebSocket;
    }

    addEventListener(type: string, listener: EventListenerOrEventListenerObject) {
      const set = this._listeners.get(type) ?? new Set();
      set.add(listener);
      this._listeners.set(type, set);
    }

    removeEventListener(type: string, listener: EventListenerOrEventListenerObject) {
      this._listeners.get(type)?.delete(listener);
    }

    dispatchEvent(_event: Event): boolean {
      return true;
    }

    send(_data: string) {}
    close() {
      this.readyState = MockWebSocket.CLOSED;
    }

    // Test helpers
    simulateOpen() {
      this.readyState = MockWebSocket.OPEN;
      const e = new Event('open');
      this.onopen?.(e);
      this._listeners.get('open')?.forEach((l) =>
        typeof l === 'function' ? l(e) : l.handleEvent(e)
      );
    }

    simulateMessage(data: object) {
      const e = new MessageEvent('message', { data: JSON.stringify(data) });
      this.onmessage?.(e);
      this._listeners.get('message')?.forEach((l) =>
        typeof l === 'function' ? l(e) : l.handleEvent(e)
      );
    }

    simulateClose() {
      this.readyState = MockWebSocket.CLOSED;
      const e = new CloseEvent('close');
      this.onclose?.(e);
      this._listeners.get('close')?.forEach((l) =>
        typeof l === 'function' ? l(e) : l.handleEvent(e)
      );
    }
  }

  let lastInstance: MockWebSocket | null = null;

  return {
    MockWebSocketClass: MockWebSocket,
    getLastInstance: () => lastInstance,
    resetInstance: () => { lastInstance = null; },
  };
});

// Apply the stub at module level (initial setup)
vi.stubGlobal('WebSocket', MockWebSocketClass);

// ──────────────────────────────────────────────
// Subject under test (imported after stub)
// ──────────────────────────────────────────────

import { WebSocketProvider, useWebSocket } from '@/contexts/websocket-context';

// ──────────────────────────────────────────────
// Wrapper factory
// ──────────────────────────────────────────────

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <WebSocketProvider>{children}</WebSocketProvider>
      </QueryClientProvider>
    );
  }
  return { Wrapper, queryClient };
}

// Helper: get the live instance (throws clearly if null)
function ws() {
  const inst = getLastInstance();
  if (!inst) throw new Error('No MockWebSocket instance — did the hook mount?');
  return inst;
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

describe('WebSocketProvider / useWebSocket', () => {
  beforeEach(() => {
    resetInstance();
    vi.useFakeTimers();
    // Re-apply stub after each test in case MSW's beforeAll overwrote it
    vi.stubGlobal('WebSocket', MockWebSocketClass);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  // ── Connection lifecycle ──────────────────────

  it('connects to websocket on mount', () => {
    const { Wrapper } = createWrapper();
    renderHook(() => useWebSocket(), { wrapper: Wrapper });

    expect(getLastInstance()).not.toBeNull();
    expect(ws().url).toMatch(/\/ws$/);
  });

  it('status is "connecting" initially', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    expect(result.current.status).toBe('connecting');
    expect(result.current.ws).toBeNull();
  });

  it('status becomes "connected" after open event', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });

    expect(result.current.status).toBe('connected');
    expect(result.current.ws).not.toBeNull();
  });

  it('status becomes "disconnected" after close event', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });
    act(() => { ws().simulateClose(); });

    expect(result.current.status).toBe('disconnected');
    expect(result.current.ws).toBeNull();
  });

  // ── Message dispatch ──────────────────────────

  it('dispatches messages to a registered listener', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });

    const callback = vi.fn();
    act(() => { result.current.subscribe('test_event', callback); });
    act(() => { ws().simulateMessage({ type: 'test_event', payload: 'hello' }); });

    expect(callback).toHaveBeenCalledOnce();
  });

  it('listener receives the correct payload', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });

    const callback = vi.fn();
    act(() => { result.current.subscribe('task_updated', callback); });

    const task = { id: 42, title: 'Test task', stage: 'planning' };
    act(() => { ws().simulateMessage({ type: 'task_updated', task }); });

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'task_updated', task })
    );
  });

  it('wildcard "*" listener receives every message type', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });

    const wildcard = vi.fn();
    act(() => { result.current.subscribe('*', wildcard); });

    act(() => {
      ws().simulateMessage({ type: 'session_started', session_id: 'abc' });
      ws().simulateMessage({ type: 'session_completed', session_id: 'abc' });
    });

    expect(wildcard).toHaveBeenCalledTimes(2);
  });

  it('unsubscribing stops the listener from receiving further messages', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });

    const callback = vi.fn();
    let unsub!: () => void;
    act(() => { unsub = result.current.subscribe('task_created', callback); });

    act(() => { ws().simulateMessage({ type: 'task_created' }); });
    expect(callback).toHaveBeenCalledTimes(1);

    act(() => { unsub(); });

    act(() => { ws().simulateMessage({ type: 'task_created' }); });
    expect(callback).toHaveBeenCalledTimes(1); // no second call
  });

  // ── Reconnect ─────────────────────────────────

  it('schedules a reconnect 3 s after close', () => {
    const { Wrapper } = createWrapper();
    renderHook(() => useWebSocket(), { wrapper: Wrapper });

    const firstInst = getLastInstance();

    act(() => {
      firstInst!.simulateOpen();
      firstInst!.simulateClose();
    });

    // Timer not yet fired — same instance
    expect(getLastInstance()).toBe(firstInst);

    act(() => { vi.advanceTimersByTime(3000); });

    // A new WebSocket should have been created
    expect(getLastInstance()).not.toBe(firstInst);
    expect(getLastInstance()).not.toBeNull();
  });

  // ── send() ────────────────────────────────────

  it('send() transmits JSON-serialised data when connected', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    act(() => { ws().simulateOpen(); });

    const sendSpy = vi.spyOn(ws(), 'send');
    act(() => { result.current.send({ action: 'ping' }); });

    expect(sendSpy).toHaveBeenCalledWith(JSON.stringify({ action: 'ping' }));
  });

  it('send() is a no-op before the connection opens', () => {
    const { Wrapper } = createWrapper();
    const { result } = renderHook(() => useWebSocket(), { wrapper: Wrapper });

    // ws is still in CONNECTING state — ws state is null on the context
    const sendSpy = vi.spyOn(ws(), 'send');
    act(() => { result.current.send({ action: 'ping' }); });

    expect(sendSpy).not.toHaveBeenCalled();
  });
});
