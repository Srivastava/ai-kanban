'use client';

import { useRef, useState } from 'react';
import { Loader2, Paperclip, Trash2, Upload } from 'lucide-react';
import { useAttachments, useDeleteAttachment, useUploadAttachment, attachmentFileUrl } from '@/hooks/use-attachments';
import type { TaskAttachment } from '@/types/attachment';

function isImage(mime: string) {
  return mime.startsWith('image/');
}

function fmtSize(bytes: number) {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
}

function AttachmentThumb({ attachment, taskId }: { attachment: TaskAttachment; taskId: string }) {
  const del = useDeleteAttachment(taskId);
  const url = attachmentFileUrl(taskId, attachment.id);

  return (
    <div className="group relative flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2 py-1.5 text-xs">
      {isImage(attachment.mime_type) ? (
        <a href={url} target="_blank" rel="noopener noreferrer" className="shrink-0">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={url} alt={attachment.filename} className="h-8 w-8 rounded object-cover" />
        </a>
      ) : (
        <Paperclip className="h-4 w-4 shrink-0 text-muted-foreground" />
      )}
      <div className="flex-1 min-w-0">
        <a href={url} target="_blank" rel="noopener noreferrer"
           className="truncate block text-foreground hover:underline max-w-[120px]">
          {attachment.filename}
        </a>
        <span className="text-muted-foreground">{fmtSize(attachment.size_bytes)}</span>
      </div>
      <button
        onClick={() => del.mutate(attachment.id)}
        disabled={del.isPending}
        className="opacity-0 group-hover:opacity-100 transition-opacity p-0.5 rounded hover:bg-destructive/10 text-muted-foreground hover:text-destructive"
        title="Remove"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

interface Props {
  taskId: string;
}

export function AttachmentZone({ taskId }: Props) {
  const { data: attachments = [] } = useAttachments(taskId);
  const upload = useUploadAttachment(taskId);
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);

  const handleFiles = (files: FileList | null) => {
    if (!files) return;
    Array.from(files).forEach((f) => upload.mutate(f));
  };

  return (
    <div className="space-y-2">
      {/* Existing attachments */}
      {attachments.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {attachments.map((a) => (
            <AttachmentThumb key={a.id} attachment={a} taskId={taskId} />
          ))}
        </div>
      )}

      {/* Upload zone */}
      <div
        className={`relative flex items-center justify-center gap-2 rounded-md border border-dashed px-3 py-2 text-xs text-muted-foreground cursor-pointer transition-colors ${
          dragging ? 'border-primary bg-primary/5' : 'border-border hover:border-primary/50 hover:bg-muted/30'
        }`}
        onClick={() => inputRef.current?.click()}
        onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
        onDragLeave={() => setDragging(false)}
        onDrop={(e) => { e.preventDefault(); setDragging(false); handleFiles(e.dataTransfer.files); }}
      >
        {upload.isPending ? (
          <>
            <Loader2 className="h-3.5 w-3.5 motion-safe:animate-spin" />
            <span>Uploading...</span>
          </>
        ) : (
          <>
            <Upload className="h-3.5 w-3.5 shrink-0" />
            <span>Attach images or files</span>
          </>
        )}
        <input
          ref={inputRef}
          type="file"
          multiple
          accept="image/*,.pdf,.txt,.md"
          className="hidden"
          onChange={(e) => handleFiles(e.target.files)}
        />
      </div>
    </div>
  );
}
