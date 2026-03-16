import { describe, it, expect, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { CreateTaskDialog } from './create-task-dialog';

vi.mock('@/lib/api-client', () => ({
  getProjects: async () => ['ai-kanban', 'my-app'],
  apiClient: vi.fn().mockResolvedValue({
    id: 'new-task-id',
    title: 'New task',
    stage: 'backlog',
    project_path: '~/Projects/my-app',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    priority: 0,
    session_id: null,
    description: null,
    instructions: null,
    context: null,
    compressed_context: null,
  }),
}));

describe('CreateTaskDialog', () => {
  it('renders dialog when open=true', () => {
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('Create New Task')).toBeInTheDocument();
  });

  it('does not render when open=false', () => {
    renderWithProviders(
      <CreateTaskDialog open={false} onOpenChange={vi.fn()} />
    );
    expect(screen.queryByText('Create New Task')).not.toBeInTheDocument();
  });

  it('disables submit button when title is empty', () => {
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );
    const submit = screen.getByRole('button', { name: /Create Task/i });
    expect(submit).toBeDisabled();
  });

  it('enables submit button when title and project are filled', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );

    await user.type(screen.getByPlaceholderText('Enter task title'), 'My task');
    await user.type(screen.getByPlaceholderText(/e\.g\. my-app/i), 'my-app');
    const submit = screen.getByRole('button', { name: /Create Task/i });
    expect(submit).not.toBeDisabled();
  });

  it('calls onOpenChange(false) on cancel', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={onOpenChange} />
    );

    await user.click(screen.getByRole('button', { name: /Cancel/i }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('submits form and calls onOpenChange(false) on success', async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={onOpenChange} />
    );

    await user.type(screen.getByPlaceholderText('Enter task title'), 'New task');
    await user.type(screen.getByPlaceholderText(/e\.g\. my-app/i), 'my-app');
    await user.click(screen.getByRole('button', { name: /Create Task/i }));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it('renders the project name input', () => {
    renderWithProviders(<CreateTaskDialog open={true} onOpenChange={vi.fn()} />);
    expect(screen.getByLabelText(/project/i)).toBeTruthy();
  });

  it('rejects path separators in project name', async () => {
    const user = userEvent.setup();
    renderWithProviders(<CreateTaskDialog open={true} onOpenChange={vi.fn()} />);
    const input = screen.getByPlaceholderText(/e\.g\. my-app/i);
    await user.type(input, '../evil');
    // sanitizeProjectName strips .. sequences on each keystroke via onChange
    expect((input as HTMLInputElement).value).not.toContain('..');
  });
});
