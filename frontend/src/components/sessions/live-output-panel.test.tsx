import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { LiveOutputPanel } from './live-output-panel';

// ---------------------------------------------------------------------------
// Mock the WebSocket context — the panel depends on subscribe/send/status
// ---------------------------------------------------------------------------
const mockSubscribe = vi.fn();
const mockSend = vi.fn();

vi.mock('@/contexts/websocket-context', () => ({
  useWebSocket: () => ({
    subscribe: mockSubscribe,
    send: mockSend,
    status: 'connected',
  }),
}));

// subscribe() must return an unsubscribe function
function makeSubscribe(handlers: Record<string, (data: unknown) => void> = {}) {
  mockSubscribe.mockImplementation((eventType: string, cb: (data: unknown) => void) => {
    handlers[eventType] = cb;
    return () => { delete handlers[eventType]; };
  });
  return handlers;
}

describe('LiveOutputPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders nothing when sessionId is empty', () => {
    makeSubscribe();
    const { container } = render(
      <LiveOutputPanel sessionId="" status="running" />
    );
    expect(container.firstChild).toBeNull();
  });

  it('shows "Waiting for output..." when running with no lines', () => {
    makeSubscribe();
    render(<LiveOutputPanel sessionId="sess-1" status="running" />);
    expect(screen.getByText('Waiting for output...')).toBeInTheDocument();
  });

  it('shows "No output captured." when not running with no lines', () => {
    makeSubscribe();
    render(<LiveOutputPanel sessionId="sess-1" status="completed" />);
    expect(screen.getByText('No output captured.')).toBeInTheDocument();
  });

  it('displays initialClaudeSessionId in header immediately', () => {
    makeSubscribe();
    render(
      <LiveOutputPanel
        sessionId="sess-1"
        status="running"
        initialClaudeSessionId="550e8400-e29b-41d4-a716-446655440000"
      />
    );
    expect(screen.getByText(/Claude session:.*550e8400-e29b-41d4-a716-446655440000/)).toBeInTheDocument();
  });

  it('updates claudeSessionId when session_id_assigned event fires', () => {
    const handlers: Record<string, (data: unknown) => void> = {};
    makeSubscribe(handlers);

    render(<LiveOutputPanel sessionId="sess-1" status="running" />);

    // Initially no claude session shown
    expect(screen.queryByText(/Claude session:/)).not.toBeInTheDocument();

    // Simulate WS event
    act(() => {
      handlers['session_id_assigned']?.({
        session_id: 'sess-1',
        claude_session_id: 'abc12345-def6-7890-ghij-klmnopqrstuv',
      });
    });

    expect(screen.getByText(/Claude session:.*abc12345-def6-7890-ghij-klmnopqrstuv/)).toBeInTheDocument();
  });

  it('ignores session_id_assigned for a different session', () => {
    const handlers: Record<string, (data: unknown) => void> = {};
    makeSubscribe(handlers);

    render(<LiveOutputPanel sessionId="sess-1" status="running" />);

    act(() => {
      handlers['session_id_assigned']?.({
        session_id: 'sess-OTHER',  // different session
        claude_session_id: 'should-not-appear',
      });
    });

    expect(screen.queryByText(/Claude session:/)).not.toBeInTheDocument();
  });

  it('subscribes to session on mount when connected', () => {
    makeSubscribe();
    render(<LiveOutputPanel sessionId="sess-1" status="running" />);
    expect(mockSend).toHaveBeenCalledWith({ type: 'subscribe_session', session_id: 'sess-1' });
  });

  it('appends output lines from session_output events', () => {
    const handlers: Record<string, (data: unknown) => void> = {};
    makeSubscribe(handlers);

    render(<LiveOutputPanel sessionId="sess-1" status="running" />);

    act(() => {
      handlers['session_output']?.({
        session_id: 'sess-1',
        output: '📖 Read: src/main.rs',
        is_error: false,
      });
    });

    expect(screen.getByText('📖 Read: src/main.rs')).toBeInTheDocument();
  });

  it('ignores session_output for a different session', () => {
    const handlers: Record<string, (data: unknown) => void> = {};
    makeSubscribe(handlers);

    render(<LiveOutputPanel sessionId="sess-1" status="running" />);

    act(() => {
      handlers['session_output']?.({
        session_id: 'sess-OTHER',
        output: 'should not appear',
        is_error: false,
      });
    });

    // Output panel should still show the "Waiting for output..." placeholder
    expect(screen.getByText('Waiting for output...')).toBeInTheDocument();
    expect(screen.queryByText('should not appear')).not.toBeInTheDocument();
  });

  it('shows rate limit banner when rate_limited event fires', () => {
    const handlers: Record<string, (data: unknown) => void> = {};
    makeSubscribe(handlers);

    render(<LiveOutputPanel sessionId="sess-1" status="running" />);

    // Fire rate_limited event with a reset time far in the future
    const resetAt = new Date(Date.now() + 30 * 60 * 1000).toISOString(); // 30 min from now
    act(() => {
      handlers['rate_limited']?.({
        session_id: 'sess-1',
        task_id: 'task-1',
        reset_at: resetAt,
      });
    });

    expect(screen.getByText(/Rate limited/)).toBeInTheDocument();
  });

  it('resets lines and claudeSessionId when sessionId prop changes', () => {
    const handlers: Record<string, (data: unknown) => void> = {};
    makeSubscribe(handlers);

    const { rerender } = render(
      <LiveOutputPanel
        sessionId="sess-1"
        status="running"
        initialClaudeSessionId="claude-aaa"
      />
    );

    expect(screen.getByText(/Claude session:.*claude-aaa/)).toBeInTheDocument();

    // Switch to a new session with no claude_session_id
    rerender(
      <LiveOutputPanel
        sessionId="sess-2"
        status="running"
        initialClaudeSessionId={null}
      />
    );

    expect(screen.queryByText(/Claude session:/)).not.toBeInTheDocument();
  });
});
