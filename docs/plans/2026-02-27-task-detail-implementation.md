# Task Detail Page Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a full-page task detail view with instructions, context, updates feed, and threaded comments for user ↔ Claude collaboration.

**Architecture:** Add context field to existing tasks table, create new task_comments table with 1-level threading, build Next.js dynamic route at /tasks/[id] with collapsible sections.

**Tech Stack:** Rust/Axum/SQLx (backend), Next.js/React Query/Shadcn (frontend)

---

## Task 1: Add Context Field to Tasks Table

**Files:**
- Create: `backend/migrations/4_add_context.sql`
- Modify: `backend/src/models/task.rs`
- Modify: `backend/src/db/tasks.rs`
- Test: `backend/tests/integration_test.rs`

### Step 1: Create migration file

```sql
-- backend/migrations/4_add_context.sql
ALTER TABLE tasks ADD COLUMN context TEXT;
```

### Step 2: Update Task model

```rust
// backend/src/models/task.rs
// Add to Task struct:
pub context: Option<String>,

// Add to UpdateTask struct:
pub context: Option<String>,

// Update Task::new() to include:
context: None,
```

### Step 3: Update repository

```rust
// backend/src/db/tasks.rs
// Update find() and list() queries to include context column
// Update update() to handle context field
```

### Step 4: Run tests

```bash
cd backend && cargo test
```
Expected: All tests pass

### Step 5: Commit

```bash
git add backend/migrations/4_add_context.sql backend/src/models/task.rs backend/src/db/tasks.rs
git commit -m "feat(backend): add context field to tasks table"
```

---

## Task 2: Create Comments Table

**Files:**
- Create: `backend/migrations/5_add_comments.sql`
- Test: `backend/tests/comment_test.rs`

### Step 1: Create migration file

```sql
-- backend/migrations/5_add_comments.sql
CREATE TABLE task_comments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES task_comments(id) ON DELETE CASCADE,
    author TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_comments_task ON task_comments(task_id);
CREATE INDEX idx_comments_parent ON task_comments(parent_id);
```

### Step 2: Verify migration runs

```bash
cd backend && cargo run &
curl http://localhost:3001/api/tasks
```
Expected: Server starts without migration errors

### Step 3: Commit

```bash
git add backend/migrations/5_add_comments.sql
git commit -m "feat(backend): add task_comments table migration"
```

---

## Task 3: Create Comment Model and Repository

**Files:**
- Modify: `backend/src/models/mod.rs`
- Create: `backend/src/models/comment.rs`
- Modify: `backend/src/db/mod.rs`
- Create: `backend/src/db/comments.rs`
- Test: `backend/tests/comment_test.rs`

### Step 1: Create Comment model

```rust
// backend/src/models/comment.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: String,
    pub task_id: String,
    pub parent_id: Option<String>,
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateComment {
    pub content: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentWithReplies {
    #[serde(flatten)]
    pub comment: Comment,
    pub replies: Vec<Comment>,
}
```

### Step 2: Export from mod.rs

```rust
// backend/src/models/mod.rs
pub mod comment;
pub use comment::{Comment, CreateComment, CommentWithReplies};
```

### Step 3: Create CommentRepository

```rust
// backend/src/db/comments.rs
use sqlx::SqlitePool;
use crate::models::{Comment, CreateComment, CommentWithReplies};
use uuid::Uuid;

pub struct CommentRepository {
    pool: SqlitePool,
}

impl CommentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_for_task(&self, task_id: &str) -> Result<Vec<CommentWithReplies>, sqlx::Error> {
        // Get all comments for task
        let all_comments = sqlx::query_as::<_, Comment>(
            "SELECT * FROM task_comments WHERE task_id = ? ORDER BY created_at ASC"
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        // Separate into top-level and replies
        let mut top_level: Vec<Comment> = Vec::new();
        let mut replies_by_parent: std::collections::HashMap<String, Vec<Comment>> = std::collections::HashMap::new();

        for comment in all_comments {
            if let Some(parent_id) = &comment.parent_id {
                replies_by_parent.entry(parent_id.clone()).or_default().push(comment);
            } else {
                top_level.push(comment);
            }
        }

        // Build result with replies
        Ok(top_level.into_iter().map(|c| {
            let replies = replies_by_parent.remove(&c.id).unwrap_or_default();
            CommentWithReplies { comment: c, replies }
        }).collect())
    }

    pub async fn create(&self, task_id: &str, author: &str, data: CreateComment) -> Result<Comment, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query_as::<_, Comment>(
            r#"INSERT INTO task_comments (id, task_id, parent_id, author, content)
               VALUES (?, ?, ?, ?, ?)
               RETURNING *"#
        )
        .bind(&id)
        .bind(task_id)
        .bind(&data.parent_id)
        .bind(author)
        .bind(&data.content)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM task_comments WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

### Step 4: Write tests

```rust
// backend/tests/comment_test.rs
use ai_kanban::{models::CreateComment, db::CommentRepository};

#[sqlx::test]
async fn test_create_comment(pool: SqlitePool) {
    let repo = CommentRepository::new(pool);

    // First create a task
    // Then create a comment
    // Assert comment is returned
}

#[sqlx::test]
async fn test_comment_with_reply(pool: SqlitePool) {
    let repo = CommentRepository::new(pool);

    // Create task, then comment, then reply
    // List comments and verify structure
}
```

### Step 5: Run tests

```bash
cd backend && cargo test comment
```
Expected: Tests pass

### Step 6: Commit

```bash
git add backend/src/models/comment.rs backend/src/models/mod.rs backend/src/db/comments.rs backend/src/db/mod.rs backend/tests/comment_test.rs
git commit -m "feat(backend): add Comment model and repository"
```

---

## Task 4: Create Comment API Endpoints

**Files:**
- Create: `backend/src/api/comments.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/api/routes.rs`
- Test: `backend/tests/comment_test.rs`

### Step 1: Create API handlers

```rust
// backend/src/api/comments.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
    routing::{get, post, delete},
};
use crate::models::{CreateComment, CommentWithReplies};
use crate::db::CommentRepository;

pub fn comment_routes() -> Router<CommentApiState> {
    Router::new()
        .route("/", get(list_comments).post(create_comment))
        .route("/:id", delete(delete_comment))
}

#[derive(Clone)]
pub struct CommentApiState {
    pub repo: CommentRepository,
}

async fn list_comments(
    State(state): State<CommentApiState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    match state.repo.list_for_task(&task_id).await {
        Ok(comments) => Json(comments).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

async fn create_comment(
    State(state): State<CommentApiState>,
    Path(task_id): Path<String>,
    Json(data): Json<CreateComment>,
) -> impl IntoResponse {
    // For now, author is "user" - later we'll detect Claude vs user
    match state.repo.create(&task_id, "user", data).await {
        Ok(comment) => (StatusCode::CREATED, Json(comment)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

async fn delete_comment(
    State(state): State<CommentApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.repo.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}
```

### Step 2: Wire up routes

```rust
// backend/src/api/routes.rs
// Add to router:
.nest("/api/tasks/:task_id/comments", comment_routes())
```

### Step 3: Test endpoints

```bash
cd backend && cargo run &
curl -X POST http://localhost:3001/api/tasks/{task_id}/comments \
  -H "Content-Type: application/json" \
  -d '{"content": "Test comment"}'
```
Expected: Comment created with 201 status

### Step 4: Commit

```bash
git add backend/src/api/comments.rs backend/src/api/mod.rs backend/src/api/routes.rs
git commit -m "feat(backend): add comment API endpoints"
```

---

## Task 5: Update Frontend Types

**Files:**
- Modify: `frontend/src/types/task.ts`
- Create: `frontend/src/types/comment.ts`

### Step 1: Update task types

```typescript
// frontend/src/types/task.ts
export interface Task {
  id: string;
  title: string;
  description: string | null;
  context: string | null;  // NEW
  stage: Stage;
  project_path: string;
  session_id: string | null;
  priority: number;
  created_at: string;
  updated_at: string;
}

export interface UpdateTask {
  title?: string;
  description?: string | null;
  context?: string | null;  // NEW
  stage?: Stage;
  priority?: number;
}
```

### Step 2: Create comment types

```typescript
// frontend/src/types/comment.ts
export type Author = 'user' | 'claude';

export interface Comment {
  id: string;
  task_id: string;
  parent_id: string | null;
  author: Author;
  content: string;
  created_at: string;
}

export interface CommentWithReplies {
  id: string;
  task_id: string;
  parent_id: string | null;
  author: Author;
  content: string;
  created_at: string;
  replies: Comment[];
}

export interface CreateComment {
  content: string;
  parent_id?: string;
}
```

### Step 3: Commit

```bash
git add frontend/src/types/task.ts frontend/src/types/comment.ts
git commit -m "feat(frontend): add context and comment types"
```

---

## Task 6: Create Comment Hooks

**Files:**
- Create: `frontend/src/hooks/use-comments.ts`

### Step 1: Create hooks

```typescript
// frontend/src/hooks/use-comments.ts
'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api-client';
import type { CommentWithReplies, CreateComment } from '@/types/comment';

export function useComments(taskId: string) {
  return useQuery({
    queryKey: ['comments', taskId],
    queryFn: () => apiClient<CommentWithReplies[]>(`/api/tasks/${taskId}/comments`),
    enabled: !!taskId,
  });
}

export function useCreateComment(taskId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateComment) =>
      apiClient<CommentWithReplies>(`/api/tasks/${taskId}/comments`, {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['comments', taskId] });
    },
  });
}

export function useDeleteComment(taskId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (commentId: string) =>
      apiClient<void>(`/api/tasks/${taskId}/comments/${commentId}`, {
        method: 'DELETE',
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['comments', taskId] });
    },
  });
}
```

### Step 2: Commit

```bash
git add frontend/src/hooks/use-comments.ts
git commit -m "feat(frontend): add comment hooks"
```

---

## Task 7: Create Task Detail Page Route

**Files:**
- Create: `frontend/src/app/tasks/[id]/page.tsx`

### Step 1: Create page component

```typescript
// frontend/src/app/tasks/[id]/page.tsx
'use client';

import { useParams, useRouter } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useTask } from '@/hooks/use-tasks';
import { TaskDetail } from '@/components/tasks/task-detail';

export default function TaskDetailPage() {
  const params = useParams();
  const router = useRouter();
  const taskId = params.id as string;

  const { data: task, isLoading, error } = useTask(taskId);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <p className="text-muted-foreground">Loading task...</p>
      </div>
    );
  }

  if (error || !task) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <p className="text-destructive">Task not found</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b border-border px-6 py-4">
        <Button variant="ghost" onClick={() => router.back()}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back
        </Button>
      </header>
      <main className="max-w-4xl mx-auto p-6">
        <TaskDetail task={task} />
      </main>
    </div>
  );
}
```

### Step 2: Commit

```bash
git add frontend/src/app/tasks/[id]/page.tsx
git commit -m "feat(frontend): add task detail page route"
```

---

## Task 8: Create TaskDetail Component

**Files:**
- Create: `frontend/src/components/tasks/task-detail.tsx`
- Create: `frontend/src/components/tasks/task-section.tsx`

### Step 1: Create TaskSection component

```typescript
// frontend/src/components/tasks/task-section.tsx
'use client';

import { useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';

interface TaskSectionProps {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
  className?: string;
}

export function TaskSection({ title, defaultOpen = true, children, className }: TaskSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className={cn('border border-border rounded-lg mb-4', className)}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 w-full px-4 py-3 text-left font-medium hover:bg-muted/50 transition-colors"
      >
        {isOpen ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        {title}
      </button>
      {isOpen && (
        <div className="px-4 pb-4">
          {children}
        </div>
      )}
    </div>
  );
}
```

### Step 2: Create TaskDetail component

```typescript
// frontend/src/components/tasks/task-detail.tsx
'use client';

import { Badge } from '@/components/ui/badge';
import { TaskSection } from './task-section';
import { CommentThread } from './comment-thread';
import { useComments } from '@/hooks/use-comments';
import type { Task, Stage } from '@/types/task';

const stageColors: Record<Stage, string> = {
  backlog: 'bg-gray-500',
  planning: 'bg-blue-500',
  ready: 'bg-yellow-500',
  in_progress: 'bg-orange-500',
  review: 'bg-purple-500',
  done: 'bg-green-500',
};

const stageLabels: Record<Stage, string> = {
  backlog: 'Backlog',
  planning: 'Planning',
  ready: 'Ready',
  in_progress: 'In Progress',
  review: 'Review',
  done: 'Done',
};

interface TaskDetailProps {
  task: Task;
}

export function TaskDetail({ task }: TaskDetailProps) {
  const { data: comments = [], isLoading: commentsLoading } = useComments(task.id);

  return (
    <div>
      <div className="flex items-start justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold">{task.title}</h1>
        <Badge className={`${stageColors[task.stage]} text-white`}>
          {stageLabels[task.stage]}
        </Badge>
      </div>

      <TaskSection title="Instructions">
        <div className="prose prose-sm dark:prose-invert max-w-none">
          {task.description || <p className="text-muted-foreground italic">No instructions yet</p>}
        </div>
      </TaskSection>

      <TaskSection title="Context">
        <div className="prose prose-sm dark:prose-invert max-w-none">
          {task.context || <p className="text-muted-foreground italic">No context added yet</p>}
        </div>
      </TaskSection>

      <TaskSection title="Updates" defaultOpen={false}>
        <p className="text-muted-foreground italic">Updates from Claude sessions will appear here</p>
      </TaskSection>

      <TaskSection title="Comments">
        {commentsLoading ? (
          <p className="text-muted-foreground">Loading comments...</p>
        ) : (
          <CommentThread taskId={task.id} comments={comments} />
        )}
      </TaskSection>
    </div>
  );
}
```

### Step 3: Commit

```bash
git add frontend/src/components/tasks/task-detail.tsx frontend/src/components/tasks/task-section.tsx
git commit -m "feat(frontend): add TaskDetail and TaskSection components"
```

---

## Task 9: Create Comment Components

**Files:**
- Create: `frontend/src/components/tasks/comment-thread.tsx`
- Create: `frontend/src/components/tasks/comment-input.tsx`

### Step 1: Create CommentInput component

```typescript
// frontend/src/components/tasks/comment-input.tsx
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
```

### Step 2: Create CommentThread component

```typescript
// frontend/src/components/tasks/comment-thread.tsx
'use client';

import { useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { Button } from '@/components/ui/button';
import { CommentInput } from './comment-input';
import type { CommentWithReplies, Comment } from '@/types/comment';

interface CommentThreadProps {
  taskId: string;
  comments: CommentWithReplies[];
}

function SingleComment({ comment, isReply = false }: { comment: Comment; isReply?: boolean }) {
  const authorLabel = comment.author === 'claude' ? 'Claude' : 'You';
  const timeAgo = formatDistanceToNow(new Date(comment.created_at), { addSuffix: true });

  return (
    <div className={`${isReply ? 'ml-8 mt-2' : 'mb-4'}`}>
      <div className="flex items-center gap-2 mb-1">
        <span className={`text-sm font-medium ${comment.author === 'claude' ? 'text-purple-600 dark:text-purple-400' : 'text-foreground'}`}>
          {authorLabel}
        </span>
        <span className="text-xs text-muted-foreground">{timeAgo}</span>
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
          <SingleComment comment={comment} />

          {/* Replies */}
          {comment.replies.map((reply) => (
            <SingleComment key={reply.id} comment={reply} isReply />
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
```

### Step 3: Install date-fns

```bash
cd frontend && npm install date-fns
```

### Step 4: Commit

```bash
git add frontend/src/components/tasks/comment-thread.tsx frontend/src/components/tasks/comment-input.tsx frontend/package.json frontend/package-lock.json
git commit -m "feat(frontend): add CommentThread and CommentInput components"
```

---

## Task 10: Link Task Cards to Detail Page

**Files:**
- Modify: `frontend/src/components/tasks/task-card.tsx`

### Step 1: Add Link wrapper

```typescript
// frontend/src/components/tasks/task-card.tsx
'use client';

import Link from 'next/link';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { Task, Stage } from '@/types/task';

// ... stageColors and stageLabels remain the same ...

interface TaskCardProps {
  task: Task;
}

export function TaskCard({ task }: TaskCardProps) {
  const createdDate = new Date(task.created_at).toLocaleDateString();

  return (
    <Link href={`/tasks/${task.id}`}>
      <Card className="hover:shadow-md transition-shadow cursor-pointer">
        {/* ... rest of card content ... */}
      </Card>
    </Link>
  );
}
```

### Step 2: Commit

```bash
git add frontend/src/components/tasks/task-card.tsx
git commit -m "feat(frontend): link task cards to detail page"
```

---

## Task 11: End-to-End Verification

### Step 1: Start backend

```bash
cd backend && cargo run
```

### Step 2: Start frontend

```bash
cd frontend && npm run dev
```

### Step 3: Test the flow

1. Open http://localhost:3002 (or available port)
2. Click on a task card
3. Verify task detail page loads
4. Verify all sections display
5. Add a comment
6. Reply to the comment
7. Navigate back to list

### Step 4: Final commit

```bash
git add -A
git commit -m "feat: complete task detail page implementation

- Add context field to tasks table
- Add task_comments table with 1-level threading
- Build task detail page at /tasks/[id]
- Add comment thread with reply functionality"
```

---

## Summary

| Task | Backend | Frontend |
|------|---------|----------|
| 1 | Context migration + model | - |
| 2 | Comments migration | - |
| 3 | Comment model + repo | - |
| 4 | Comment API endpoints | - |
| 5 | - | Types |
| 6 | - | Comment hooks |
| 7 | - | Page route |
| 8 | - | TaskDetail component |
| 9 | - | Comment components |
| 10 | - | Link cards |
| 11 | - | E2E verification |
