import { describe, it, expect, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { CreateTaskDialog } from './create-task-dialog';

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

  it('enables submit button when title is filled', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <CreateTaskDialog open={true} onOpenChange={vi.fn()} />
    );

    await user.type(screen.getByPlaceholderText('Enter task title'), 'My task');
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
    await user.click(screen.getByRole('button', { name: /Create Task/i }));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });
});
