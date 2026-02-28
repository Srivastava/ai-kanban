'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { useCreateComment } from '@/hooks/use-comments';

interface CommentInputProps {
  taskId: string;
  parentId?: string;
  onSuccess?: () => void;
  placeholder?: string;
}

export function CommentInput({ taskId, parentId, onSuccess, placeholder = "Add a comment..." }: CommentInputProps) {
  const [content, setContent] = useState('');
  const createComment = useCreateComment(taskId);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim()) return;

    await createComment.mutateAsync({
      content: content.trim(),
      parent_id: parentId,
    });

    setContent('');
    onSuccess?.();
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2">
      <Textarea
        value={content}
        onChange={(e) => setContent(e.target.value)}
        placeholder={placeholder}
        rows={2}
        className="flex-1"
      />
      <Button type="submit" disabled={!content.trim() || createComment.isPending}>
        {createComment.isPending ? 'Sending...' : 'Send'}
      </Button>
    </form>
  );
}
