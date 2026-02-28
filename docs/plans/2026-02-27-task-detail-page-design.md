# Task Detail Page Design

**Date:** 2026-02-27
**Status:** Approved
**Related:** Sprint 4 Frontend Foundation

## Overview

A full-page task detail view with structured sections for instructions, context, updates, and comments. Enables async collaboration between users and Claude.

## Route

**URL:** `/tasks/[id]`

- Click task card → navigate to detail page
- Back button returns to task list
- Breadcrumb navigation: `Home > Task Title`

## Page Layout

```
┌─────────────────────────────────────────────────────────┐
│  ← Back    Task Title                    [Stage Badge]  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─ Instructions ─────────────────────────────────────┐ │
│  │ What to do (markdown, editable by user)            │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ Context ──────────────────────────────────────────┐ │
│  │ Current state, constraints, key files              │ │
│  │ User + Claude can update                           │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ Updates Feed ─────────────────────────────────────┐ │
│  │ Feb 27 3pm - Claude: Discovered API endpoint...    │ │
│  │ Feb 27 2pm - Claude: Added auth module             │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ Comments ─────────────────────────────────────────┐ │
│  │ User: Can you also handle error cases?             │ │
│  │   └─ Claude: Yes, I'll add try-catch blocks        │ │
│  │ [Add comment input]                                │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## Section Details

### 1. Instructions
- **Source:** Reuses existing `description` field from tasks table
- **Format:** Markdown
- **Editable by:** User only
- **Purpose:** Define what the task should accomplish

### 2. Context
- **Source:** New `context` field in tasks table
- **Format:** Markdown
- **Editable by:** User + Claude can suggest updates
- **Purpose:** Current state, constraints, relevant files, decisions made
- **Constraint:** Keep concise (~1-2 pages) to avoid token bloat

### 3. Updates Feed
- **Source:** Stored as comments with `author = 'claude'` and special type
- **Format:** Timestamped entries
- **Editable by:** Claude auto-appends during sessions
- **Purpose:** Session log of discoveries, progress, decisions

### 4. Comments
- **Source:** `task_comments` table
- **Format:** 1-level threaded comments
- **Participants:** User ↔ Claude discussion
- **Purpose:** Async collaboration, questions, clarifications

## Data Model

### Migration 1: Add context to tasks

```sql
ALTER TABLE tasks ADD COLUMN context TEXT;
```

### Migration 2: Create comments table

```sql
CREATE TABLE task_comments (
    id TEXT PRIMARY KEY,                    -- UUID v4
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES task_comments(id) ON DELETE CASCADE,  -- null = top-level
    author TEXT NOT NULL,                   -- 'user' | 'claude'
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_comments_task ON task_comments(task_id);
CREATE INDEX idx_comments_parent ON task_comments(parent_id);
```

### Threading Rules

- `parent_id = null` → top-level comment
- `parent_id = <id>` → reply to that comment
- Only 1 level deep (replies cannot have replies)
- Deleting parent deletes children (CASCADE)

## API Endpoints

### Task Endpoints (extended)

```
GET    /api/tasks/:id          # Include context field in response
PATCH  /api/tasks/:id          # Accept { context: string }
```

### Comment Endpoints (new)

```
GET    /api/tasks/:id/comments           # List all comments with replies
POST   /api/tasks/:id/comments           # Create comment
        Body: { content: string, parent_id?: string }
DELETE /api/comments/:id                 # Delete comment (and children)
```

### Response Format

```json
{
  "comments": [
    {
      "id": "uuid",
      "task_id": "uuid",
      "parent_id": null,
      "author": "claude",
      "content": "Ready for review",
      "created_at": "2026-02-27T15:00:00Z",
      "replies": [
        {
          "id": "uuid",
          "parent_id": "uuid",
          "author": "user",
          "content": "Looks good",
          "created_at": "2026-02-27T15:05:00Z"
        }
      ]
    }
  ]
}
```

## Frontend Components

| Component | Path | Purpose |
|-----------|------|---------|
| TaskDetailPage | `app/tasks/[id]/page.tsx` | Page wrapper, data fetching |
| TaskDetail | `components/tasks/task-detail.tsx` | Main layout container |
| TaskSection | `components/tasks/task-section.tsx` | Reusable collapsible section |
| CommentThread | `components/tasks/comment-thread.tsx` | Comments with 1-level replies |
| CommentInput | `components/tasks/comment-input.tsx` | Add new comment form |

## User Flows

### Flow 1: View Task Details
1. User clicks task card in list
2. Navigate to `/tasks/[id]`
3. Page loads task with all sections
4. User can expand/collapse sections

### Flow 2: Add Comment
1. User types in comment input
2. Clicks "Send"
3. Comment appears immediately (optimistic update)
4. API persists to database

### Flow 3: Reply to Claude
1. Claude session ends with question in comments
2. User sees notification indicator
3. User opens task, sees Claude's comment
4. User clicks "Reply" on that comment
5. Types response, submits
6. Next Claude session reads reply and continues

## Implementation Order

1. Backend: Add context field + migration
2. Backend: Create comments table + migration
3. Backend: Implement comment API endpoints
4. Frontend: Create task detail page route
5. Frontend: Build TaskDetail layout component
6. Frontend: Build TaskSection component
7. Frontend: Build CommentThread component
8. Frontend: Build CommentInput component
9. Frontend: Wire up API calls with React Query
10. Integration: Update task cards to link to detail page

## Future Enhancements (Out of Scope)

- Markdown editor with preview
- @mentions in comments
- Comment reactions
- Real-time comment updates via WebSocket
- Email notifications for new comments
