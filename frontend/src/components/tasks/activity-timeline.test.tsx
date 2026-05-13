import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { ActivityTimeline } from './activity-timeline';
import type { Task } from '@/types/task';

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock('@/contexts/websocket-context');
vi.mock('@/hooks/use-comments');
vi.mock('@/hooks/use-sessions');

import { useWebSocket } from '@/contexts/websocket-context';
import { useComments } from '@/hooks/use-comments';
import { useTaskSessionsDetail } from '@/hooks/use-sessions';

// ── Helpers ───────────────────────────────────────────────────────────────────

const mockTask: Task = {
  id: 'task-1',
  title: 'Test task',
  description: null,
  stage: 'in_progress',
  project_path: '~/Projects/test',
  session_id: null,
  priority: 0,
  instructions: null,
  context: null,
  compressed_context: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

function setupWebSocketMock() {
  const callbacks: Record<string, (data: unknown) => void> = {};
  (useWebSocket as ReturnType<typeof vi.fn>).mockReturnValue({
    subscribe: (type: string, cb: (data: unknown) => void) => {
      callbacks[type] = cb;
      return () => { delete callbacks[type]; };
    },
  });
  return callbacks;
}

function renderTimeline(sessions: { id: string; started_at: string; ended_at: string | null; error_message: string | null }[] = []) {
  (useComments as ReturnType<typeof vi.fn>).mockReturnValue({ data: [], isLoading: false });
  (useTaskSessionsDetail as ReturnType<typeof vi.fn>).mockReturnValue({ data: sessions });
  return render(<ActivityTimeline task={mockTask} sessionId={null} />);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('ActivityTimeline — session_failed entries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows retrying description for live session_failed event with will_retry=true', async () => {
    const callbacks = setupWebSocketMock();
    renderTimeline();

    act(() => {
      callbacks['session_failed']?.({
        task_id: 'task-1',
        session_id: 'sess-1',
        retry_attempt: 0,
        max_retries: 3,
        will_retry: true,
      });
    });

    expect(screen.getByText('Session failed — retrying (attempt 1 of 3)')).toBeInTheDocument();
  });

  it('shows max-retries description for live session_failed event with will_retry=false', async () => {
    const callbacks = setupWebSocketMock();
    renderTimeline();

    act(() => {
      callbacks['session_failed']?.({
        task_id: 'task-1',
        session_id: 'sess-1',
        retry_attempt: 3,
        max_retries: 3,
        will_retry: false,
      });
    });

    expect(screen.getByText('Session failed — max retries reached')).toBeInTheDocument();
  });

  it('ignores session_failed events for other tasks', async () => {
    const callbacks = setupWebSocketMock();
    renderTimeline();

    act(() => {
      callbacks['session_failed']?.({
        task_id: 'other-task',
        session_id: 'sess-2',
        retry_attempt: 0,
        max_retries: 3,
        will_retry: true,
      });
    });

    expect(screen.queryByText(/Session failed/)).not.toBeInTheDocument();
  });

  it('shows historical retry entry from error_message "failed:retry:0"', () => {
    setupWebSocketMock();
    renderTimeline([
      {
        id: 'sess-hist-1',
        started_at: '2026-01-02T10:00:00Z',
        ended_at: '2026-01-02T10:05:00Z',
        error_message: 'failed:retry:0',
      },
    ]);

    expect(screen.getByText('Session failed — retrying (attempt 1 of 3)')).toBeInTheDocument();
  });

  it('shows historical exhausted entry from error_message "failed:exhausted"', () => {
    setupWebSocketMock();
    renderTimeline([
      {
        id: 'sess-hist-2',
        started_at: '2026-01-02T10:10:00Z',
        ended_at: '2026-01-02T10:15:00Z',
        error_message: 'failed:exhausted',
      },
    ]);

    expect(screen.getByText('Session failed — max retries reached')).toBeInTheDocument();
  });
});
