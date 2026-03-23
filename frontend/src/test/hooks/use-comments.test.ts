import { describe, it, expect } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useComments, useCreateComment, useDeleteComment } from '@/hooks/use-comments';
import type { CommentWithReplies } from '@/types/comment';

function wrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

const mockComment: CommentWithReplies = {
  id: 'comment-1',
  task_id: 'task-123',
  parent_id: null,
  author: 'user',
  content: 'This is a test comment',
  created_at: '2026-03-01T10:00:00Z',
  replies: [],
};

describe('useComments', () => {
  it('returns empty array when no comments exist', async () => {
    // Default handler returns []
    const { result } = renderHook(() => useComments('task-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(0);
  });

  it('returns comments for a task', async () => {
    server.use(
      http.get('/api/tasks/:id/comments', () =>
        HttpResponse.json([mockComment])
      )
    );
    const { result } = renderHook(() => useComments('task-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(1);
    expect(result.current.data![0].content).toBe('This is a test comment');
  });

  it('is disabled when taskId is empty string', () => {
    const { result } = renderHook(() => useComments(''), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });

  it('returns error state on API failure', async () => {
    server.use(
      http.get('/api/tasks/:id/comments', () =>
        HttpResponse.json({ error: 'Server error' }, { status: 500 })
      )
    );
    const { result } = renderHook(() => useComments('task-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });

  it('returns comments with replies', async () => {
    const reply = { ...mockComment, id: 'reply-1', parent_id: 'comment-1' };
    const commentWithReply = { ...mockComment, replies: [reply] };
    server.use(
      http.get('/api/tasks/:id/comments', () =>
        HttpResponse.json([commentWithReply])
      )
    );
    const { result } = renderHook(() => useComments('task-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data![0].replies).toHaveLength(1);
  });
});

describe('useCreateComment', () => {
  it('has mutate function', () => {
    server.use(
      http.post('/api/tasks/:id/comments', () =>
        HttpResponse.json(mockComment, { status: 201 })
      )
    );
    const { result } = renderHook(() => useCreateComment('task-123'), { wrapper: wrapper() });
    expect(typeof result.current.mutate).toBe('function');
  });

  it('creates a comment successfully', async () => {
    server.use(
      http.post('/api/tasks/:id/comments', () =>
        HttpResponse.json(mockComment, { status: 201 })
      )
    );
    const { result } = renderHook(() => useCreateComment('task-123'), { wrapper: wrapper() });
    result.current.mutate({ content: 'New comment' });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.content).toBe('This is a test comment');
  });

  it('enters error state on API failure', async () => {
    server.use(
      http.post('/api/tasks/:id/comments', () =>
        HttpResponse.json({ error: 'Bad request' }, { status: 400 })
      )
    );
    const { result } = renderHook(() => useCreateComment('task-123'), { wrapper: wrapper() });
    result.current.mutate({ content: '' });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe('useDeleteComment', () => {
  it('has mutate function', () => {
    const { result } = renderHook(() => useDeleteComment('task-123'), { wrapper: wrapper() });
    expect(typeof result.current.mutate).toBe('function');
  });

  it('is in idle state initially', () => {
    const { result } = renderHook(() => useDeleteComment('task-123'), { wrapper: wrapper() });
    expect(result.current.isIdle).toBe(true);
  });
});
