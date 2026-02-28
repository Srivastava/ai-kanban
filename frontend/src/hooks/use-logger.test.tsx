import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useLogger } from './use-logger';

describe('useLogger', () => {
  it('returns a ContextLogger with debug/info/warn/error methods', () => {
    const { result } = renderHook(() => useLogger({ target: 'TestComponent' }));
    expect(typeof result.current.debug).toBe('function');
    expect(typeof result.current.info).toBe('function');
    expect(typeof result.current.warn).toBe('function');
    expect(typeof result.current.error).toBe('function');
  });

  it('does not recreate logger when options are stable', () => {
    const { result, rerender } = renderHook(() =>
      useLogger({ target: 'Stable', taskId: 'task-1' })
    );
    const first = result.current;
    rerender();
    // Same reference because useMemo dependencies didn't change
    expect(result.current).toBe(first);
  });

  it('recreates logger when taskId changes', () => {
    let taskId = 'task-1';
    const { result, rerender } = renderHook(() => useLogger({ taskId }));
    const first = result.current;
    taskId = 'task-2';
    rerender();
    // Different reference because taskId changed
    expect(result.current).not.toBe(first);
  });
});
