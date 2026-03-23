import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import type { Task, Stage } from '@/types/task';

// Mock dnd-kit — not available in jsdom
vi.mock('@dnd-kit/core', () => ({
  DndContext: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  DragOverlay: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  PointerSensor: class {},
  useSensor: vi.fn(() => ({})),
  useSensors: vi.fn(() => []),
  useDroppable: () => ({ setNodeRef: vi.fn(), isOver: false }),
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

// Mock hooks
const mockUpdateTask = vi.fn();
vi.mock('@/hooks/use-tasks', () => ({
  useUpdateTask: () => ({ mutate: mockUpdateTask, isPending: false }),
  useDeleteTask: () => ({ mutate: vi.fn(), isPending: false }),
  useMoveTask: () => ({ mutate: vi.fn() }),
}));

vi.mock('@/hooks/use-sessions', () => ({
  useAllSessions: () => ({ data: [] }),
}));

// Mock apiClient used for cost data query
vi.mock('@/lib/api-client', () => ({
  apiClient: vi.fn(() => Promise.resolve([])),
}));

// ── helpers ────────────────────────────────────────────────────────────────────

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

// ── tests ──────────────────────────────────────────────────────────────────────

describe('KanbanBoard', () => {
  beforeEach(() => {
    mockUpdateTask.mockReset();
  });

  it('renders all stage column headers', () => {
    renderWithProviders(<KanbanBoard tasks={[]} />);
    // All 6 stage labels should be visible
    expect(screen.getAllByText('Backlog').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Planning').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Ready').length).toBeGreaterThan(0);
    expect(screen.getAllByText('In Progress').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Review').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Done').length).toBeGreaterThan(0);
  });

  it('shows tasks in correct columns', () => {
    const tasks = [
      makeTask({ id: 't1', title: 'Backlog task', stage: 'backlog' }),
      makeTask({ id: 't2', title: 'Planning task', stage: 'planning' }),
      makeTask({ id: 't3', title: 'Done task', stage: 'done' }),
    ];
    renderWithProviders(<KanbanBoard tasks={tasks} />);
    expect(screen.getByText('Backlog task')).toBeInTheDocument();
    expect(screen.getByText('Planning task')).toBeInTheDocument();
    expect(screen.getByText('Done task')).toBeInTheDocument();
  });

  it('shows total task count in stats bar', () => {
    const tasks = [
      makeTask({ id: 't1', title: 'Task 1', stage: 'backlog' }),
      makeTask({ id: 't2', title: 'Task 2', stage: 'planning' }),
      makeTask({ id: 't3', title: 'Task 3', stage: 'done' }),
    ];
    renderWithProviders(<KanbanBoard tasks={tasks} />);
    // Stats bar shows "3 tasks"
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('tasks')).toBeInTheDocument();
  });

  it('shows tasks label in stats bar when no tasks provided', () => {
    renderWithProviders(<KanbanBoard tasks={[]} />);
    // Stats bar renders "0 tasks" — the word "tasks" should be present
    expect(screen.getByText('tasks')).toBeInTheDocument();
  });

  it('shows hide empty / show empty toggle button', () => {
    renderWithProviders(<KanbanBoard tasks={[]} />);
    // Initially "Hide empty" button should be present
    expect(screen.getByRole('button', { name: /hide empty/i })).toBeInTheDocument();
  });

  it('toggles to show empty when hide empty is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<KanbanBoard tasks={[]} />);
    const toggleBtn = screen.getByRole('button', { name: /hide empty/i });
    await user.click(toggleBtn);
    // After click, should switch to "Show empty"
    expect(screen.getByRole('button', { name: /show empty/i })).toBeInTheDocument();
  });

  it('shows in_progress count in stats bar when tasks exist', () => {
    const tasks: Task[] = [
      makeTask({ id: 't1', title: 'Task 1', stage: 'in_progress' }),
      makeTask({ id: 't2', title: 'Task 2', stage: 'in_progress' }),
    ];
    renderWithProviders(<KanbanBoard tasks={tasks} />);
    expect(screen.getByText('in progress')).toBeInTheDocument();
  });

  it('does not show in_progress stat when no in_progress tasks', () => {
    const tasks = [makeTask({ id: 't1', title: 'Task 1', stage: 'backlog' })];
    renderWithProviders(<KanbanBoard tasks={tasks} />);
    expect(screen.queryByText('in progress')).not.toBeInTheDocument();
  });

  it('renders tasks correctly when multiple tasks in same column', () => {
    const tasks = [
      makeTask({ id: 't1', title: 'First backlog task', stage: 'backlog' }),
      makeTask({ id: 't2', title: 'Second backlog task', stage: 'backlog' }),
      makeTask({ id: 't3', title: 'Third backlog task', stage: 'backlog' }),
    ];
    renderWithProviders(<KanbanBoard tasks={tasks} />);
    expect(screen.getByText('First backlog task')).toBeInTheDocument();
    expect(screen.getByText('Second backlog task')).toBeInTheDocument();
    expect(screen.getByText('Third backlog task')).toBeInTheDocument();
  });

  it('accepts onCreateTask callback prop without error', () => {
    const onCreateTask = vi.fn();
    expect(() =>
      renderWithProviders(<KanbanBoard tasks={[]} onCreateTask={onCreateTask} />)
    ).not.toThrow();
  });

  it('renders loading state without crashing', () => {
    renderWithProviders(<KanbanBoard tasks={[]} isLoading={true} />);
    // Board stats bar still renders
    expect(screen.getByText('tasks')).toBeInTheDocument();
  });
});
