import { describe, it, expect, vi } from 'vitest';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { renderWithProviders } from '@/test/utils';
import { mockTask } from '@/test/msw/fixtures';
import type { Task } from '@/types/task';

// Mock next/navigation (used by Link)
vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn() }),
  usePathname: () => '/',
  useSearchParams: () => new URLSearchParams(),
}));

// Import after mocks
import { TaskCard } from '@/components/tasks/task-card';

// Default: no active sessions
beforeEach(() => {
  server.use(
    http.get('/api/sessions/all', () => HttpResponse.json([]))
  );
});

const baseTask: Task = {
  ...mockTask,
  stage: 'backlog',
  priority: 0,
};

describe('TaskCard', () => {
  it('renders task title', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByText('Test task')).toBeInTheDocument();
  });

  it('renders stage badge', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByText('Backlog')).toBeInTheDocument();
  });

  it('renders description preview', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByText('A test task description')).toBeInTheDocument();
  });

  it('renders project folder name', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByText('project')).toBeInTheDocument();
  });

  it('renders updated time', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByText(/updated/i)).toBeInTheDocument();
  });

  it('renders delete button', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByRole('button', { name: /delete task/i })).toBeInTheDocument();
  });

  it('renders more options button', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    expect(screen.getByRole('button', { name: /more options/i })).toBeInTheDocument();
  });

  it('shows move menu on more options click', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TaskCard task={baseTask} />);
    await user.click(screen.getByRole('button', { name: /more options/i }));
    // Should show move options (not for current stage)
    expect(screen.getByText(/open task/i)).toBeInTheDocument();
  });

  it('opens confirm delete dialog on delete click', async () => {
    const user = userEvent.setup();
    renderWithProviders(<TaskCard task={baseTask} />);
    await user.click(screen.getByRole('button', { name: /delete task/i }));
    // Dialog should appear
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('renders priority badge for high priority', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, priority: 3 }} />);
    expect(screen.getByText('High')).toBeInTheDocument();
  });

  it('renders priority badge for critical priority', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, priority: 4 }} />);
    expect(screen.getByText('Critical')).toBeInTheDocument();
  });

  it('does not render priority badge for zero priority', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, priority: 0 }} />);
    expect(screen.queryByText('Low')).not.toBeInTheDocument();
  });

  it('shows In Progress stage badge', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, stage: 'in_progress' }} />);
    expect(screen.getByText('In Progress')).toBeInTheDocument();
  });

  it('does not show active session indicator when no session', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, session_id: null }} />);
    expect(screen.queryByLabelText('Session active')).not.toBeInTheDocument();
  });

  it('filters move menu options to exclude current stage', async () => {
    const user = userEvent.setup();
    // Task is already in_progress
    renderWithProviders(<TaskCard task={{ ...baseTask, stage: 'in_progress' }} />);
    await user.click(screen.getByRole('button', { name: /more options/i }));
    // Should not show "Move to In Progress" since already there
    expect(screen.queryByText('Move to In Progress')).not.toBeInTheDocument();
  });

  it('renders card as a link to task detail', () => {
    renderWithProviders(<TaskCard task={baseTask} />);
    const links = screen.getAllByRole('link');
    const taskLink = links.find((l) => l.getAttribute('href') === `/tasks/${baseTask.id}`);
    expect(taskLink).toBeDefined();
  });

  it('handles task with no description', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, description: null }} />);
    // Should not throw and task title should still appear
    expect(screen.getByText('Test task')).toBeInTheDocument();
  });

  it('handles task with no project path', () => {
    renderWithProviders(<TaskCard task={{ ...baseTask, project_path: '' }} />);
    // No folder chip — just ensure no crash
    expect(screen.getByText('Test task')).toBeInTheDocument();
  });
});
