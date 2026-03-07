'use client';

import { useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CommentInput } from './comment-input';
import { useDeleteComment } from '@/hooks/use-comments';
import type { CommentWithReplies, Comment } from '@/types/comment';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface CommentThreadProps {
  taskId: string;
  comments: CommentWithReplies[];
}

function SingleComment({ comment, isReply = false, taskId }: { comment: Comment; isReply?: boolean; taskId: string }) {
  const authorLabel = comment.author === 'claude' ? 'Claude' : 'You';
  const timeAgo = formatDistanceToNow(new Date(comment.created_at), { addSuffix: true });
  const { mutate: deleteComment, isPending } = useDeleteComment(taskId);

  return (
    <div className={`${isReply ? 'ml-8 mt-2' : 'mb-4'} group`}>
      <div className="flex items-center gap-2 mb-1">
        <span className={`text-sm font-medium ${comment.author === 'claude' ? 'text-purple-600 dark:text-purple-400' : 'text-foreground'}`}>
          {authorLabel}
        </span>
        <span className="text-xs text-muted-foreground">{timeAgo}</span>
        {comment.author !== 'claude' && (
          <button
            onClick={() => deleteComment(comment.id)}
            disabled={isPending}
            className="ml-auto opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"
            title="Delete comment"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
      <div className="text-sm [&_p]:mb-2 [&_p:last-child]:mb-0 [&_ul]:list-disc [&_ul]:pl-4 [&_ul]:mb-2 [&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:mb-2 [&_li]:mb-1 [&_h1]:text-base [&_h1]:font-bold [&_h1]:mb-2 [&_h2]:text-sm [&_h2]:font-semibold [&_h2]:mb-1 [&_h3]:text-sm [&_h3]:font-semibold [&_h3]:mb-1 [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-xs [&_code]:font-mono [&_pre]:bg-muted [&_pre]:p-3 [&_pre]:rounded [&_pre]:overflow-x-auto [&_pre]:mb-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_blockquote]:border-l-2 [&_blockquote]:border-border [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:text-muted-foreground [&_a]:text-primary [&_a]:underline [&_hr]:border-border [&_strong]:font-semibold [&_em]:italic">
        <ReactMarkdown remarkPlugins={[remarkGfm]}>{comment.content}</ReactMarkdown>
      </div>
    </div>
  );
}

export function CommentThread({ taskId, comments }: CommentThreadProps) {
  const [replyingTo, setReplyingTo] = useState<string | null>(null);

  if (comments.length === 0) {
    return (
      <div>
        <p className="text-muted-foreground italic mb-4">No comments yet. Start the discussion!</p>
        <CommentInput taskId={taskId} />
      </div>
    );
  }

  const sorted = [...comments].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

  return (
    <div className="space-y-4">
      {/* Add new top-level comment */}
      <div className="pb-4 border-b border-border">
        <CommentInput taskId={taskId} />
      </div>

      {sorted.map((comment) => (
        <div key={comment.id} className="border-b border-border pb-4 last:border-0">
          <SingleComment comment={comment} taskId={taskId} />

          {/* Replies */}
          {comment.replies.map((reply) => (
            <SingleComment key={reply.id} comment={reply} isReply taskId={taskId} />
          ))}

          {/* Reply input */}
          {replyingTo === comment.id ? (
            <div className="ml-8 mt-2">
              <CommentInput
                taskId={taskId}
                parentId={comment.id}
                placeholder="Write a reply..."
                onSuccess={() => setReplyingTo(null)}
              />
              <Button variant="ghost" size="sm" onClick={() => setReplyingTo(null)} className="mt-2">
                Cancel
              </Button>
            </div>
          ) : (
            <Button variant="ghost" size="sm" onClick={() => setReplyingTo(comment.id)} className="ml-8 mt-2">
              Reply
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}
