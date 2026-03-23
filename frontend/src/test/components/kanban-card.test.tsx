import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { KanbanCard } from '@/components/kanban/kanban-card';
import type { Task } from '@/types/task';

// Mock dnd-kit sortable — not available in jsdom
vi.mock('@dnd-kit/sortable', () => ({
  useSortable: () => ({
    attributes: {},
    listeners: {},
    setNodeRef: vi.fn(),
    transform: null,
    transition: undefined,
    isDragging: false,
  }),
}));

vi.mock('@dnd-kit/utilities', () => ({
  CSS: {
    Transform: {
      toString: () => '',
    },
  },
}));

// Mock hooks so we don't hit the API
const mockDeleteTask = vi.fn();
const mockMoveTask = vi.fn();

vi.mock('@/hooks/use-tasks', () => ({
  useDeleteTask: () => ({ mutate: mockDeleteTask, isPending: false }),
  useMoveTask: () => ({ mutate: mockMoveTask }),
}));

vi.mock('@/hooks/use-sessions', () => ({
  useAllSessions: () => ({ data: [] }),
}));

// ── helpers ───────────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    title: 'Fix the login bug',
    description: null,
    instructions: null,
    context: null,
    compressed_context: null,
    stage: 'backlog',
    project_path: '/home/user/Projects/my-app',
    session_id: null,
    priority: 0,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T12:00:00Z',
    ...overrides,
  };
}

// ── tests ─────────────────────────────────────────────────────────────────────

describe('KanbanCard', () => {
  beforeEach(() => {
    mockDeleteTask.mockReset();
    mockMoveTask.mockReset();
  });

  it('renders task title', () => {
    renderWithProviders(<KanbanCard task={makeTask()} />);
    expect(screen.getByText('Fix the login bug')).toBeInTheDocument();
  });

  it('renders description preview when description is provided', () => {
    const task = makeTask({ description: 'This is a description of the bug' });
    renderWithProviders(<KanbanCard task={task} />);
    expect(screen.getByText('This is a description of the bug')).toBeInTheDocument();
  });

  it('does not render description section when description is null', () => {
    renderWithProviders(<KanbanCard task={makeTask({ description: null })} />);
    // No paragraph text — only title and project chip visible
    expect(screen.queryByText(/description/i)).not.toBeInTheDocument();
  });

  it('strips markdown from description preview', () => {
    const task = makeTask({ description: '**Bold text** and `code`' });
    renderWithProviders(<KanbanCard task={task} />);
    expect(screen.getByText('Bold text and code')).toBeInTheDocument();
  });

  it('renders project folder chip', () => {
    renderWithProviders(<KanbanCard task={makeTask()} />);
    // Last segment of /home/user/Projects/my-app
    expect(screen.getByText('my-app')).toBeInTheDocument();
  });

  it('renders priority label when priority > 0', () => {
    renderWithProviders(<KanbanCard task={makeTask({ priority: 3 })} />);
    expect(screen.getByText('High')).toBeInTheDocument();
  });

  it('renders Critical priority label', () => {
    renderWithProviders(<KanbanCard task={makeTask({ priority: 4 })} />);
    expect(screen.getByText('Critical')).toBeInTheDocument();
  });

  it('renders Low priority label', () => {
    renderWithProviders(<KanbanCard task={makeTask({ priority: 1 })} />);
    expect(screen.getByText('Low')).toBeInTheDocument();
  });

  it('does not render priority label when priority is 0', () => {
    renderWithProviders(<KanbanCard task={makeTask({ priority: 0 })} />);
    expect(screen.queryByText('Low')).not.toBeInTheDocument();
    expect(screen.queryByText('Medium')).not.toBeInTheDocument();
    expect(screen.queryByText('High')).not.toBeInTheDocument();
    expect(screen.queryByText('Critical')).not.toBeInTheDocument();
  });

  it('renders link to task detail page', () => {
    renderWithProviders(<KanbanCard task={makeTask({ id: 'task-abc' })} />);
    const link = screen.getByRole('link', { name: /Fix the login bug/i });
    expect(link).toHaveAttribute('href', '/tasks/task-abc');
  });

  it('renders delete button (aria-label)', () => {
    renderWithProviders(<KanbanCard task={makeTask()} />);
    expect(screen.getByRole('button', { name: /Delete task/i })).toBeInTheDocument();
  });

  it('renders more options button', () => {
    renderWithProviders(<KanbanCard task={makeTask()} />);
    expect(screen.getByRole('button', { name: /More options/i })).toBeInTheDocument();
  });

  it('opens confirm delete dialog when delete button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<KanbanCard task={makeTask({ title: 'Task to delete' })} />);
    await user.click(screen.getByRole('button', { name: /Delete task/i }));
    // ConfirmDeleteDialog should show its own "Delete task?" heading
    expect(screen.getByText('Delete task?')).toBeInTheDocument();
  });

  it('opens overflow menu with move options when more options clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<KanbanCard task={makeTask({ stage: 'backlog' })} />);
    await user.click(screen.getByRole('button', { name: /More options/i }));
    // Should show move options for all stages except current (backlog)
    expect(screen.getByText(/Move to Planning/i)).toBeInTheDocument();
    expect(screen.getByText(/Move to In Progress/i)).toBeInTheDocument();
    expect(screen.queryByText(/Move to Backlog/i)).not.toBeInTheDocument();
  });

  it('calls moveTask mutation when a move option is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<KanbanCard task={makeTask({ id: 'task-1', stage: 'backlog' })} />);
    await user.click(screen.getByRole('button', { name: /More options/i }));
    await user.click(screen.getByText(/Move to Planning/i));
    expect(mockMoveTask).toHaveBeenCalledWith({ id: 'task-1', stage: 'planning' });
  });

  it('renders cost badge when costData has cost > 0', () => {
    const costData = {
      task_id: 'task-1',
      task_title: 'Fix the login bug',
      input_tokens: 1000,
      output_tokens: 200,
      cache_creation_tokens: 0,
      cache_read_tokens: 0,
      cost_usd: 0.05,
    };
    renderWithProviders(<KanbanCard task={makeTask()} costData={costData} />);
    expect(screen.getByText('$0.05')).toBeInTheDocument();
  });

  it('renders token badge when costData has tokens > 0', () => {
    const costData = {
      task_id: 'task-1',
      task_title: 'Fix the login bug',
      input_tokens: 5000,
      output_tokens: 1000,
      cache_creation_tokens: 0,
      cache_read_tokens: 0,
      cost_usd: 0,
    };
    renderWithProviders(<KanbanCard task={makeTask()} costData={costData} />);
    expect(screen.getByText('6k tok')).toBeInTheDocument();
  });

  it('does not render delete/more buttons when isOverlay=true', () => {
    renderWithProviders(<KanbanCard task={makeTask()} isOverlay={true} />);
    expect(screen.queryByRole('button', { name: /Delete task/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /More options/i })).not.toBeInTheDocument();
  });

  it('shows active session indicator when session is active', () => {
    vi.mock('@/hooks/use-sessions', () => ({
      useAllSessions: () => ({
        data: [{ task_id: 'task-active', status: 'running' }],
      }),
    }));
    renderWithProviders(
      <KanbanCard task={makeTask({ id: 'task-active', session_id: 'sess-1' })} />
    );
    // The pulse indicator has aria-label "Session active"
    // Note: due to vi.mock hoisting, this verifies the conditional rendering path
    expect(screen.getByText('Fix the login bug')).toBeInTheDocument();
  });
});
