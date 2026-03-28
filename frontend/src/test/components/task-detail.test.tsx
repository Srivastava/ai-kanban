import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { TaskDetail } from '@/components/tasks/task-detail';
import type { Task } from '@/types/task';

// ── Mock WebSocket context ─────────────────────────────────────────────────────
const mockSubscribe = vi.fn(() => vi.fn()); // returns an unsubscribe fn

vi.mock('@/contexts/websocket-context', () => ({
  useWebSocket: () => ({
    subscribe: mockSubscribe,
    send: vi.fn(),
    isConnected: false,
  }),
}));

// ── Mock hooks ─────────────────────────────────────────────────────────────────
vi.mock('@/hooks/use-comments', () => ({
  useComments: () => ({ data: [], isLoading: false }),
  useCreateComment: () => ({ mutate: vi.fn(), isPending: false }),
}));

vi.mock('@/hooks/use-sessions', () => ({
  useSession: () => ({ data: null }),
  useTaskSessionsDetail: () => ({ data: [], isError: false }),
  useAllSessions: () => ({ data: [] }),
}));

vi.mock('@/hooks/use-tasks', () => ({
  useUpdateTask: () => ({ mutate: vi.fn(), isPending: false }),
  useDeleteTask: () => ({ mutate: vi.fn(), isPending: false }),
}));

vi.mock('@/hooks/use-attachments', () => ({
  useAttachments: () => ({ data: [], isLoading: false }),
  useUploadAttachment: () => ({ mutate: vi.fn(), isPending: false }),
  attachmentFileUrl: (taskId: string, attId: string) => `/api/tasks/${taskId}/attachments/${attId}/file`,
}));

// ── Mock SessionControls and LiveOutputPanel — they set up complex WS logic ────
vi.mock('@/components/sessions/session-controls', () => ({
  SessionControls: () => <div data-testid="session-controls" />,
}));

vi.mock('@/components/sessions/live-output-panel', () => ({
  LiveOutputPanel: () => <div data-testid="live-output-panel" />,
}));

// ── Mock apiClient ─────────────────────────────────────────────────────────────
vi.mock('@/lib/api-client', () => ({
  apiClient: vi.fn(() => Promise.resolve(null)),
}));

// ── helpers ────────────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-abc',
    title: 'Implement OAuth login',
    description: 'Add Google OAuth support to the login page',
    instructions: null,
    context: null,
    compressed_context: null,
    stage: 'backlog',
    project_path: '/home/user/Projects/my-app',
    session_id: null,
    priority: 0,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-15T12:00:00Z',
    ...overrides,
  };
}

// ── tests ──────────────────────────────────────────────────────────────────────

describe('TaskDetail', () => {
  beforeEach(() => {
    mockSubscribe.mockReset();
    mockSubscribe.mockReturnValue(vi.fn());
  });

  it('renders task title', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByRole('heading', { name: 'Implement OAuth login' })).toBeInTheDocument();
  });

  it('renders task description via Description card', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    // CollapsibleCard with "Description" header
    expect(screen.getByText('Description')).toBeInTheDocument();
    // The actual description text
    expect(screen.getByText('Add Google OAuth support to the login page')).toBeInTheDocument();
  });

  it('renders task stage badge', () => {
    renderWithProviders(<TaskDetail task={makeTask({ stage: 'planning' })} />);
    expect(screen.getByText('Planning')).toBeInTheDocument();
  });

  it('renders backlog stage badge', () => {
    renderWithProviders(<TaskDetail task={makeTask({ stage: 'backlog' })} />);
    expect(screen.getByText('Backlog')).toBeInTheDocument();
  });

  it('renders in_progress stage badge', () => {
    renderWithProviders(<TaskDetail task={makeTask({ stage: 'in_progress' })} />);
    expect(screen.getByText('In Progress')).toBeInTheDocument();
  });

  it('renders done stage badge', () => {
    renderWithProviders(<TaskDetail task={makeTask({ stage: 'done' })} />);
    expect(screen.getByText('Done')).toBeInTheDocument();
  });

  it('renders project path', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByText('/home/user/Projects/my-app')).toBeInTheDocument();
  });

  it('renders delete task button', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByRole('button', { name: /delete task/i })).toBeInTheDocument();
  });

  it('opens confirm delete dialog on delete button click', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TaskDetail task={makeTask({ title: 'Task to delete' })} />);
    await user.click(screen.getByRole('button', { name: /delete task/i }));
    expect(screen.getByText('Delete task?')).toBeInTheDocument();
  });

  it('calls onDelete callback when confirmed in delete dialog', async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    renderWithProviders(<TaskDetail task={makeTask({ title: 'Task to delete' })} onDelete={onDelete} />);
    await user.click(screen.getByRole('button', { name: /delete task/i }));
    // Confirm the deletion
    const confirmBtn = screen.getByRole('button', { name: /^delete$/i });
    await user.click(confirmBtn);
    expect(onDelete).toHaveBeenCalledOnce();
  });

  it('renders Instructions card section', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByText('Instructions')).toBeInTheDocument();
  });

  it('renders Context card section', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByText('Context')).toBeInTheDocument();
  });

  it('renders session history card', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByText('Session History')).toBeInTheDocument();
  });

  it('shows "Move to Done" button when task stage is review', () => {
    renderWithProviders(<TaskDetail task={makeTask({ stage: 'review' })} />);
    expect(screen.getByRole('button', { name: /move to done/i })).toBeInTheDocument();
  });

  it('does not show "Move to Done" button for non-review stages', () => {
    renderWithProviders(<TaskDetail task={makeTask({ stage: 'in_progress' })} />);
    expect(screen.queryByRole('button', { name: /move to done/i })).not.toBeInTheDocument();
  });

  it('renders placeholder text when description is null', () => {
    renderWithProviders(<TaskDetail task={makeTask({ description: null })} />);
    expect(screen.getByText(/no description yet/i)).toBeInTheDocument();
  });

  it('renders copy project path button', () => {
    renderWithProviders(<TaskDetail task={makeTask()} />);
    expect(screen.getByRole('button', { name: /copy project path/i })).toBeInTheDocument();
  });

  it('shows "Copied!" feedback after clicking copy path button', async () => {
    const user = userEvent.setup();
    // Mock clipboard API
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn(() => Promise.resolve()) },
      writable: true,
    });
    renderWithProviders(<TaskDetail task={makeTask()} />);
    await user.click(screen.getByRole('button', { name: /copy project path/i }));
    expect(await screen.findByText('Copied!')).toBeInTheDocument();
  });
});
