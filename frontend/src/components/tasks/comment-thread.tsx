'use client';

import { useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { CommentInput } from './comment-input';
import { useDeleteComment } from '@/hooks/use-comments';
import type { CommentWithReplies, Comment } from '@/types/comment';

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
      <p className="text-sm whitespace-pre-wrap">{comment.content}</p>
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

  return (
    <div className="space-y-4">
      {comments.map((comment) => (
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

      {/* Add new top-level comment */}
      <div className="pt-4 border-t border-border">
        <CommentInput taskId={taskId} />
      </div>
    </div>
  );
}
