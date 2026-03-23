import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/msw/server';
import { CommentInput } from '@/components/tasks/comment-input';

// Mock the hooks that use react-query mutations
vi.mock('@/hooks/use-comments', () => ({
  useCreateComment: vi.fn(() => ({
    mutateAsync: vi.fn().mockResolvedValue({ id: 'comment-1', content: 'hello', author: 'user' }),
    isPending: false,
  })),
}));

vi.mock('@/hooks/use-attachments', () => ({
  attachmentFileUrl: (taskId: string, attachmentId: string) =>
    `/api/tasks/${taskId}/attachments/${attachmentId}/file`,
}));

describe('CommentInput', () => {
  it('renders textarea', () => {
    renderWithProviders(<CommentInput taskId="task-1" />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('submit button disabled when empty', () => {
    renderWithProviders(<CommentInput taskId="task-1" />);
    expect(screen.getByRole('button', { name: /send/i })).toBeDisabled();
  });

  it('submit button enabled when text entered', async () => {
    const user = userEvent.setup();
    renderWithProviders(<CommentInput taskId="task-1" />);
    await user.type(screen.getByRole('textbox'), 'Hello world');
    expect(screen.getByRole('button', { name: /send/i })).not.toBeDisabled();
  });

  it('calls onSubmit with text', async () => {
    const { useCreateComment } = await import('@/hooks/use-comments');
    const mutateAsync = vi.fn().mockResolvedValue({ id: 'c1', content: 'Hello', author: 'user' });
    vi.mocked(useCreateComment).mockReturnValue({ mutateAsync, isPending: false } as ReturnType<typeof useCreateComment>);

    const user = userEvent.setup();
    renderWithProviders(<CommentInput taskId="task-1" />);
    await user.type(screen.getByRole('textbox'), 'Hello');
    await user.click(screen.getByRole('button', { name: /send/i }));

    await waitFor(() => {
      expect(mutateAsync).toHaveBeenCalledWith({ content: 'Hello', parent_id: undefined });
    });
  });

  it('clears input after submit', async () => {
    const { useCreateComment } = await import('@/hooks/use-comments');
    const mutateAsync = vi.fn().mockResolvedValue({ id: 'c1', content: 'Hello', author: 'user' });
    vi.mocked(useCreateComment).mockReturnValue({ mutateAsync, isPending: false } as ReturnType<typeof useCreateComment>);

    const user = userEvent.setup();
    renderWithProviders(<CommentInput taskId="task-1" />);
    const textarea = screen.getByRole('textbox');
    await user.type(textarea, 'Hello');
    await user.click(screen.getByRole('button', { name: /send/i }));

    await waitFor(() => {
      expect((textarea as HTMLTextAreaElement).value).toBe('');
    });
  });

  it('renders with custom placeholder', () => {
    renderWithProviders(<CommentInput taskId="task-1" placeholder="Write a reply..." />);
    expect(screen.getByPlaceholderText('Write a reply...')).toBeInTheDocument();
  });

  it('renders with default placeholder', () => {
    renderWithProviders(<CommentInput taskId="task-1" />);
    expect(screen.getByPlaceholderText('Add a comment...')).toBeInTheDocument();
  });

  it('calls onSuccess callback after submit', async () => {
    const { useCreateComment } = await import('@/hooks/use-comments');
    const mutateAsync = vi.fn().mockResolvedValue({ id: 'c1', content: 'Hello', author: 'user' });
    vi.mocked(useCreateComment).mockReturnValue({ mutateAsync, isPending: false } as ReturnType<typeof useCreateComment>);

    const onSuccess = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<CommentInput taskId="task-1" onSuccess={onSuccess} />);
    await user.type(screen.getByRole('textbox'), 'Hello');
    await user.click(screen.getByRole('button', { name: /send/i }));

    await waitFor(() => {
      expect(onSuccess).toHaveBeenCalled();
    });
  });

  it('does not submit when only whitespace entered', async () => {
    const { useCreateComment } = await import('@/hooks/use-comments');
    const mutateAsync = vi.fn().mockResolvedValue({});
    vi.mocked(useCreateComment).mockReturnValue({ mutateAsync, isPending: false } as ReturnType<typeof useCreateComment>);

    const user = userEvent.setup();
    renderWithProviders(<CommentInput taskId="task-1" />);
    await user.type(screen.getByRole('textbox'), '   ');
    expect(screen.getByRole('button', { name: /send/i })).toBeDisabled();
  });

  it('shows hint text about image paste', () => {
    renderWithProviders(<CommentInput taskId="task-1" />);
    expect(screen.getByText(/paste or drag images/i)).toBeInTheDocument();
  });
});
