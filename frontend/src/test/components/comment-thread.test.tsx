import { describe, it, expect, vi } from 'vitest';
import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { CommentThread } from '@/components/tasks/comment-thread';
import type { CommentWithReplies } from '@/types/comment';

vi.mock('@/hooks/use-comments', () => ({
  useCreateComment: vi.fn(() => ({
    mutateAsync: vi.fn().mockResolvedValue({}),
    isPending: false,
  })),
  useDeleteComment: vi.fn(() => ({
    mutate: vi.fn(),
    isPending: false,
  })),
}));

vi.mock('@/hooks/use-attachments', () => ({
  attachmentFileUrl: (taskId: string, attachmentId: string) =>
    `/api/tasks/${taskId}/attachments/${attachmentId}/file`,
}));

const makeComment = (overrides: Partial<CommentWithReplies> = {}): CommentWithReplies => ({
  id: 'comment-1',
  task_id: 'task-1',
  parent_id: null,
  author: 'user',
  content: 'This is a comment',
  created_at: new Date().toISOString(),
  replies: [],
  ...overrides,
});

describe('CommentThread', () => {
  it('renders empty state for no comments', () => {
    renderWithProviders(<CommentThread taskId="task-1" comments={[]} />);
    expect(screen.getByText(/no comments yet/i)).toBeInTheDocument();
  });

  it('renders comment input in empty state', () => {
    renderWithProviders(<CommentThread taskId="task-1" comments={[]} />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('renders comments list', () => {
    const comments = [
      makeComment({ id: 'c1', content: 'First comment' }),
      makeComment({ id: 'c2', content: 'Second comment' }),
    ];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    expect(screen.getByText('First comment')).toBeInTheDocument();
    expect(screen.getByText('Second comment')).toBeInTheDocument();
  });

  it('renders author name for user comment', () => {
    const comments = [makeComment({ author: 'user' })];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    expect(screen.getByText('You')).toBeInTheDocument();
  });

  it('renders author name for claude comment', () => {
    const comments = [makeComment({ author: 'claude', content: 'AI response' })];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    expect(screen.getByText('Claude')).toBeInTheDocument();
  });

  it('renders comment content', () => {
    const comments = [makeComment({ content: 'Hello from user' })];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    expect(screen.getByText('Hello from user')).toBeInTheDocument();
  });

  it('renders timestamp', () => {
    const comments = [makeComment({ created_at: new Date().toISOString() })];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    // formatDistanceToNow produces something like "less than a minute ago"
    expect(screen.getByText(/ago/i)).toBeInTheDocument();
  });

  it('renders reply button for each comment', () => {
    const comments = [makeComment()];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    expect(screen.getByRole('button', { name: /reply/i })).toBeInTheDocument();
  });

  it('shows reply input when Reply is clicked', async () => {
    const user = userEvent.setup();
    const comments = [makeComment()];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    await user.click(screen.getByRole('button', { name: /reply/i }));
    expect(screen.getByPlaceholderText('Write a reply...')).toBeInTheDocument();
  });

  it('hides reply input when Cancel is clicked', async () => {
    const user = userEvent.setup();
    const comments = [makeComment()];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    await user.click(screen.getByRole('button', { name: /reply/i }));
    expect(screen.getByPlaceholderText('Write a reply...')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /cancel/i }));
    expect(screen.queryByPlaceholderText('Write a reply...')).not.toBeInTheDocument();
  });

  it('renders replies nested under parent comment', () => {
    const comments = [
      makeComment({
        id: 'parent-1',
        content: 'Parent comment',
        replies: [
          {
            id: 'reply-1',
            task_id: 'task-1',
            parent_id: 'parent-1',
            author: 'claude',
            content: 'Reply from Claude',
            created_at: new Date().toISOString(),
          },
        ],
      }),
    ];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    expect(screen.getByText('Parent comment')).toBeInTheDocument();
    expect(screen.getByText('Reply from Claude')).toBeInTheDocument();
  });

  it('does not show delete button for claude comments', () => {
    const comments = [makeComment({ author: 'claude', content: 'AI comment' })];
    renderWithProviders(<CommentThread taskId="task-1" comments={comments} />);
    // delete button should not be present for claude comments
    expect(screen.queryByTitle('Delete comment')).not.toBeInTheDocument();
  });
});
