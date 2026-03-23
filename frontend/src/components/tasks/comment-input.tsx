'use client';

import { useState, useCallback } from 'react';
import { X, ImageIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { useCreateComment } from '@/hooks/use-comments';
import { attachmentFileUrl } from '@/hooks/use-attachments';

interface PendingImage {
  file: File;
  previewUrl: string;
}

interface CommentInputProps {
  taskId: string;
  parentId?: string;
  onSuccess?: () => void;
  placeholder?: string;
}

export function CommentInput({ taskId, parentId, onSuccess, placeholder = 'Add a comment...' }: CommentInputProps) {
  const [content, setContent] = useState('');
  const [pendingImages, setPendingImages] = useState<PendingImage[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const createComment = useCreateComment(taskId);

  const addImages = useCallback((files: FileList | File[]) => {
    const images = Array.from(files).filter((f) => f.type.startsWith('image/'));
    if (images.length === 0) return;
    setPendingImages((prev) => [
      ...prev,
      ...images.map((file) => ({ file, previewUrl: URL.createObjectURL(file) })),
    ]);
  }, []);

  const handlePaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items) return;
    const imageFiles = Array.from(items)
      .filter((i) => i.kind === 'file' && i.type.startsWith('image/'))
      .map((i) => i.getAsFile())
      .filter((f): f is File => f !== null);
    if (imageFiles.length > 0) {
      e.preventDefault();
      addImages(imageFiles);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    addImages(e.dataTransfer.files);
  };

  const removeImage = (index: number) => {
    setPendingImages((prev) => {
      URL.revokeObjectURL(prev[index].previewUrl);
      return prev.filter((_, i) => i !== index);
    });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim() && pendingImages.length === 0) return;

    let finalContent = content.trim();

    // Upload pending images and build markdown
    if (pendingImages.length > 0) {
      const markdownParts: string[] = [];
      for (const { file } of pendingImages) {
        const form = new FormData();
        form.append('file', file);
        const res = await fetch(`/api/tasks/${taskId}/attachments`, { method: 'POST', body: form });
        if (res.ok) {
          const att = await res.json() as { id: string; filename: string };
          markdownParts.push(`![${att.filename}](${attachmentFileUrl(taskId, att.id)})`);
        }
      }
      if (markdownParts.length > 0) {
        finalContent = finalContent
          ? `${finalContent}\n\n${markdownParts.join('\n')}`
          : markdownParts.join('\n');
      }
      pendingImages.forEach((p) => URL.revokeObjectURL(p.previewUrl));
    }

    if (!finalContent.trim()) return;

    await createComment.mutateAsync({
      content: finalContent,
      parent_id: parentId,
    });

    setContent('');
    setPendingImages([]);
    onSuccess?.();
  };

  const canSubmit = (!!content.trim() || pendingImages.length > 0) && !createComment.isPending;

  return (
    <form onSubmit={handleSubmit} className="space-y-2">
      <div
        className={`relative rounded-md transition-colors ${isDragging ? 'ring-2 ring-primary/50 bg-primary/5' : ''}`}
        onDragOver={(e) => { e.preventDefault(); setIsDragging(true); }}
        onDragLeave={() => setIsDragging(false)}
        onDrop={handleDrop}
      >
        <Textarea
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onPaste={handlePaste}
          placeholder={isDragging ? 'Drop image here…' : placeholder}
          rows={2}
          className="w-full resize-none"
        />
        {isDragging && (
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none rounded-md">
            <span className="flex items-center gap-2 text-sm text-primary/70 font-medium">
              <ImageIcon className="h-4 w-4" />
              Drop to attach
            </span>
          </div>
        )}
      </div>

      {pendingImages.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {pendingImages.map((img, i) => (
            <div key={i} className="relative group">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={img.previewUrl}
                alt={img.file.name}
                className="h-16 w-16 object-cover rounded border border-border"
              />
              <button
                type="button"
                onClick={() => removeImage(i)}
                className="absolute -top-1.5 -right-1.5 h-4 w-4 rounded-full bg-background border border-border flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-destructive hover:border-destructive hover:text-destructive-foreground"
              >
                <X className="h-2.5 w-2.5" />
              </button>
            </div>
          ))}
        </div>
      )}

      <div className="flex items-center justify-between">
        <p className="text-[10px] text-muted-foreground">
          Paste or drag images to attach
        </p>
        <Button type="submit" size="sm" disabled={!canSubmit}>
          {createComment.isPending ? 'Sending…' : 'Send'}
        </Button>
      </div>
    </form>
  );
}
