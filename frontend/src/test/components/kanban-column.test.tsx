import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { KanbanColumn } from '@/components/kanban/kanban-column';
import type { Task, Stage } from '@/types/task';

// Mock dnd-kit — not available in jsdom
vi.mock('@dnd-kit/core', () => ({
  useDroppable: () => ({
    setNodeRef: vi.fn(),
    isOver: false,
  }),
  DndContext: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@dnd-kit/sortable', () => ({
  SortableContext: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  verticalListSortingStrategy: {},
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

// Mock hooks used by KanbanCard (rendered inside KanbanColumn)
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

function makeTask(id: string, title: string, stage: Stage = 'backlog', overrides: Partial<Task> = {}): Task {
  return {
    id,
    title,
    description: null,
    instructions: null,
    context: null,
    compressed_context: null,
    stage,
    project_path: '/home/user/Projects/my-app',
    session_id: null,
    priority: 0,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-15T00:00:00Z',
    ...overrides,
  };
}

// ── tests ─────────────────────────────────────────────────────────────────────

describe('KanbanColumn', () => {
  beforeEach(() => {
    mockDeleteTask.mockReset();
    mockMoveTask.mockReset();
  });

  it('renders column title for backlog stage', () => {
    renderWithProviders(<KanbanColumn stage="backlog" tasks={[]} />);
    expect(screen.getByText('Backlog')).toBeInTheDocument();
  });

  it('renders column title for in_progress stage', () => {
    renderWithProviders(<KanbanColumn stage="in_progress" tasks={[]} />);
    expect(screen.getByText('In Progress')).toBeInTheDocument();
  });

  it('renders column title for all stages', () => {
    const stages: Stage[] = ['backlog', 'planning', 'ready', 'in_progress', 'review', 'done'];
    const labels = ['Backlog', 'Planning', 'Ready', 'In Progress', 'Review', 'Done'];
    stages.forEach((stage, i) => {
      const { unmount } = renderWithProviders(<KanbanColumn stage={stage} tasks={[]} />);
      expect(screen.getByText(labels[i])).toBeInTheDocument();
      unmount();
    });
  });

  it('renders task count badge showing 0 when no tasks', () => {
    renderWithProviders(<KanbanColumn stage="backlog" tasks={[]} />);
    expect(screen.getByText('0')).toBeInTheDocument();
  });

  it('renders task count badge showing correct count', () => {
    const tasks = [
      makeTask('t1', 'Task one', 'backlog'),
      makeTask('t2', 'Task two', 'backlog'),
      makeTask('t3', 'Task three', 'backlog'),
    ];
    renderWithProviders(<KanbanColumn stage="backlog" tasks={tasks} />);
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('renders task cards for tasks in the column', () => {
    const tasks = [
      makeTask('t1', 'First task', 'planning'),
      makeTask('t2', 'Second task', 'planning'),
    ];
    renderWithProviders(<KanbanColumn stage="planning" tasks={tasks} />);
    expect(screen.getByText('First task')).toBeInTheDocument();
    expect(screen.getByText('Second task')).toBeInTheDocument();
  });

  it('renders empty state (add task button) when no tasks', () => {
    renderWithProviders(<KanbanColumn stage="backlog" tasks={[]} />);
    expect(screen.getByText('Add task')).toBeInTheDocument();
  });

  it('does not render empty-state button when tasks are present', () => {
    const tasks = [makeTask('t1', 'A task', 'ready')];
    renderWithProviders(<KanbanColumn stage="ready" tasks={tasks} />);
    expect(screen.queryByText('Add task')).not.toBeInTheDocument();
  });

  it('renders add-task button in header when onCreateTask is provided', () => {
    const onCreateTask = vi.fn();
    renderWithProviders(
      <KanbanColumn stage="planning" tasks={[]} onCreateTask={onCreateTask} />
    );
    expect(screen.getAllByRole('button', { name: /Add task to Planning/i })[0]).toBeInTheDocument();
  });

  it('does not render add button in header when onCreateTask is not provided', () => {
    // Pass a task so the empty-state button is not rendered; only the header button matters here
    renderWithProviders(<KanbanColumn stage="planning" tasks={[makeTask('t1', 'Task 1', 'planning')]} />);
    expect(screen.queryAllByRole('button', { name: /Add task to Planning/i })).toHaveLength(0);
  });

  it('calls onCreateTask when header add button is clicked', async () => {
    const user = userEvent.setup();
    const onCreateTask = vi.fn();
    renderWithProviders(
      <KanbanColumn stage="ready" tasks={[]} onCreateTask={onCreateTask} />
    );
    await user.click(screen.getAllByRole('button', { name: /Add task to Ready/i })[0]);
    expect(onCreateTask).toHaveBeenCalledWith('ready');
  });

  it('calls onCreateTask when empty-state add task button is clicked', async () => {
    const user = userEvent.setup();
    const onCreateTask = vi.fn();
    renderWithProviders(
      <KanbanColumn stage="backlog" tasks={[]} onCreateTask={onCreateTask} />
    );
    await user.click(screen.getByText('Add task'));
    expect(onCreateTask).toHaveBeenCalledWith('backlog');
  });

  it('renders loading skeletons when isLoading=true', () => {
    renderWithProviders(<KanbanColumn stage="backlog" tasks={[]} isLoading={true} />);
    // Empty state button should not appear during loading
    expect(screen.queryByText('Add task')).not.toBeInTheDocument();
  });

  it('shows WIP warning when in_progress column exceeds 3 tasks', () => {
    const tasks = [
      makeTask('t1', 'Task 1', 'in_progress'),
      makeTask('t2', 'Task 2', 'in_progress'),
      makeTask('t3', 'Task 3', 'in_progress'),
      makeTask('t4', 'Task 4', 'in_progress'),
    ];
    renderWithProviders(<KanbanColumn stage="in_progress" tasks={tasks} />);
    expect(screen.getByText('WIP')).toBeInTheDocument();
  });

  it('does not show WIP warning when in_progress column is within limit', () => {
    const tasks = [
      makeTask('t1', 'Task 1', 'in_progress'),
      makeTask('t2', 'Task 2', 'in_progress'),
    ];
    renderWithProviders(<KanbanColumn stage="in_progress" tasks={tasks} />);
    expect(screen.queryByText('WIP')).not.toBeInTheDocument();
  });

  it('does not show WIP warning for backlog stage regardless of task count', () => {
    const tasks = Array.from({ length: 10 }, (_, i) =>
      makeTask(`t${i}`, `Task ${i}`, 'backlog')
    );
    renderWithProviders(<KanbanColumn stage="backlog" tasks={tasks} />);
    expect(screen.queryByText('WIP')).not.toBeInTheDocument();
  });

  it('renders older tasks section divider for done column with old tasks', () => {
    const now = new Date();
    const oldDate = new Date(now.getTime() - 10 * 24 * 60 * 60 * 1000).toISOString(); // 10 days ago
    const tasks = [makeTask('t1', 'Old task', 'done', { updated_at: oldDate })];
    renderWithProviders(<KanbanColumn stage="done" tasks={tasks} />);
    expect(screen.getByText(/Older than 7d/i)).toBeInTheDocument();
  });

  it('does not render older tasks divider for done column with only recent tasks', () => {
    const recentDate = new Date().toISOString();
    const tasks = [makeTask('t1', 'Recent task', 'done', { updated_at: recentDate })];
    renderWithProviders(<KanbanColumn stage="done" tasks={tasks} />);
    expect(screen.queryByText(/Older than/i)).not.toBeInTheDocument();
  });

  it('does not render older tasks divider for non-done columns', () => {
    const oldDate = new Date(Date.now() - 20 * 24 * 60 * 60 * 1000).toISOString();
    const tasks = [makeTask('t1', 'Old-ish task', 'backlog', { updated_at: oldDate })];
    renderWithProviders(<KanbanColumn stage="backlog" tasks={tasks} />);
    expect(screen.queryByText(/Older than/i)).not.toBeInTheDocument();
  });
});
