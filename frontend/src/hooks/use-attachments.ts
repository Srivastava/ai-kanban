'use client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type { TaskAttachment } from '@/types/attachment';

export function useAttachments(taskId: string) {
  return useQuery({
    queryKey: ['attachments', taskId],
    queryFn: () => apiClient<TaskAttachment[]>(`/api/tasks/${taskId}/attachments`),
    enabled: !!taskId,
  });
}

export function useUploadAttachment(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (file: File) => {
      const form = new FormData();
      form.append('file', file);
      const res = await fetch(`/api/tasks/${taskId}/attachments`, {
        method: 'POST',
        body: form,
      });
      if (!res.ok) throw new Error('Upload failed');
      return res.json() as Promise<TaskAttachment>;
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['attachments', taskId] }),
  });
}

export function useDeleteAttachment(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (attachmentId: string) =>
      fetch(`/api/tasks/${taskId}/attachments/${attachmentId}`, { method: 'DELETE' }).then(() => {}),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['attachments', taskId] }),
  });
}

export function attachmentFileUrl(taskId: string, attachmentId: string) {
  return `/api/tasks/${taskId}/attachments/${attachmentId}/file`;
}
