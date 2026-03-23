import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement } from 'react';
import { useAttachments, useUploadAttachment, useDeleteAttachment, attachmentFileUrl } from '@/hooks/use-attachments';
import type { TaskAttachment } from '@/types/attachment';

function wrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

const mockAttachment: TaskAttachment = {
  id: 'att-1',
  task_id: 'task-123',
  filename: 'screenshot.png',
  mime_type: 'image/png',
  storage_path: '/uploads/att-1-screenshot.png',
  created_at: '2026-03-01T10:00:00Z',
};

describe('useAttachments', () => {
  beforeEach(() => {
    server.use(
      http.get('/api/tasks/:id/attachments', () =>
        HttpResponse.json([mockAttachment])
      )
    );
  });

  it('returns attachments for a task', async () => {
    const { result } = renderHook(() => useAttachments('task-123'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(1);
    expect(result.current.data![0].filename).toBe('screenshot.png');
  });

  it('is disabled when taskId is empty', () => {
    const { result } = renderHook(() => useAttachments(''), { wrapper: wrapper() });
    expect(result.current.isFetching).toBe(false);
  });

  it('returns empty array when no attachments', async () => {
    server.use(
      http.get('/api/tasks/:id/attachments', () => HttpResponse.json([]))
    );
    const { result } = renderHook(() => useAttachments('task-456'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(0);
  });

  it('returns error state on API failure', async () => {
    server.use(
      http.get('/api/tasks/:id/attachments', () =>
        HttpResponse.json({ error: 'Server error' }, { status: 500 })
      )
    );
    const { result } = renderHook(() => useAttachments('task-fail'), { wrapper: wrapper() });
    await waitFor(() => expect(result.current.isError).toBe(true));
  });
});

describe('useDeleteAttachment', () => {
  it('has a mutate function', () => {
    const { result } = renderHook(() => useDeleteAttachment('task-123'), { wrapper: wrapper() });
    expect(typeof result.current.mutate).toBe('function');
  });

  it('is idle initially', () => {
    const { result } = renderHook(() => useDeleteAttachment('task-123'), { wrapper: wrapper() });
    expect(result.current.isIdle).toBe(true);
  });

  it('calls delete endpoint successfully', async () => {
    server.use(
      http.delete('/api/tasks/:taskId/attachments/:attId', () =>
        new HttpResponse(null, { status: 204 })
      )
    );
    const { result } = renderHook(() => useDeleteAttachment('task-123'), { wrapper: wrapper() });
    act(() => { result.current.mutate('att-1'); });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});

describe('attachmentFileUrl', () => {
  it('returns the correct file URL', () => {
    const url = attachmentFileUrl('task-123', 'att-456');
    expect(url).toBe('/api/tasks/task-123/attachments/att-456/file');
  });

  it('handles different task and attachment IDs', () => {
    const url = attachmentFileUrl('my-task', 'my-attachment');
    expect(url).toContain('my-task');
    expect(url).toContain('my-attachment');
  });
});

describe('useUploadAttachment', () => {
  it('has a mutate function', () => {
    const { result } = renderHook(() => useUploadAttachment('task-123'), { wrapper: wrapper() });
    expect(typeof result.current.mutate).toBe('function');
  });

  it('is idle initially', () => {
    const { result } = renderHook(() => useUploadAttachment('task-123'), { wrapper: wrapper() });
    expect(result.current.isIdle).toBe(true);
  });
});
