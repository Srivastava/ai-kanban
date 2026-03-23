# Pipeline Audit, Bug Fixes, and Test Coverage Design

## Goal

Audit and fix bugs in the context/attachment/image → Claude/LiteLLM pipeline, then bring backend test coverage to 90% and set up + achieve 90% frontend unit test coverage using Vitest.

## Execution Order

Three sequential sub-plans:
1. **Plan 1** — Pipeline bug audit and fixes
2. **Plan 2** — Backend test coverage to 90%
3. **Plan 3** — Frontend Vitest setup and unit test coverage to 90%

---

## Sub-plan 1: Pipeline Bug Audit and Fixes

### Scope

The pipeline under audit is: user uploads attachment/image → stored on disk + DB → session starts → files copied to `.claude/attachments/` → context file written → LiteLLM receives images for summarization/enrichment.

### Modules to Audit

#### 1. `backend/src/api/attachments.rs` — Upload Handler

**Filename sanitization** (`upload_attachment`):
- Current: replaces `/`, `\`, and `..` but does not strip path traversal edge cases like `....//`
- Current: no max filename length check
- Current: no file size limit enforced in handler (relies on axum body limit)
- **Fix**: use a strict allowlist sanitizer — keep only alphanumeric, `.`, `-`, `_`; truncate to 255 chars

**MIME type detection**:
- Current: trusts `content_type` from the multipart field (client-supplied, unverified)
- Potential bug: browser may send wrong MIME or empty string
- **Fix**: if `content_type` is empty or `application/octet-stream`, infer from file extension using a small lookup table

**Filename collision on upload**:
- Current: storage path is `{dir}/{uuid}-{safe_name}` — the UUID prefix makes collisions impossible at storage level ✓
- No bug here

#### 2. `backend/src/claude/manager.rs` — `start_session()` Attachment Copy

**Filename collision when copying to `.claude/attachments/`**:
- Current: copies `{storage_path}` → `{project}/.claude/attachments/{att.filename}` using the *original* filename (no UUID prefix)
- **Bug**: if two attachments have the same filename (e.g., two `screenshot.png` files uploaded at different times), the second copy silently overwrites the first
- **Fix**: use `{att.id}-{att.filename}` as the destination filename; update context file to match

**Context file attachment list**:
- Current writes: `.claude/attachments/{att.filename}`
- After fix, must write: `.claude/attachments/{att.id}-{att.filename}`

#### 3. `backend/src/ai/litellm.rs` — `image_to_data_url()`

**MIME type inferred from extension, not from stored `mime_type`**:
- Current: `image_to_data_url(path)` takes only a path, infers MIME from extension (`.png` → `image/png`, default → `image/jpeg`)
- **Bug**: a file stored as `photo.jpg` but uploaded with `image/webp` content-type will be encoded with wrong MIME (`image/jpeg` instead of `image/webp`)
- The stored `TaskAttachment.mime_type` field has the correct value but is not passed to this function
- **Fix**: change signature to `image_to_data_url(path: &str, mime_type: &str) -> Option<String>` and use the provided MIME instead of inferring from extension
- Update all callers in `context_manager.rs`

#### 4. `backend/src/ai/context_manager.rs` — `task_image_data_urls()`

**Caller passes only path, loses MIME**:
- Current: `image_to_data_url(&att.storage_path)` — does not pass `att.mime_type`
- **Fix**: after `image_to_data_url` signature change, pass `&att.mime_type`

#### 5. `backend/src/claude/manager.rs` — `write_task_context_file()`

**Comment filtering**:
- Current: `c.comment.author != "litellm"` correctly excludes LiteLLM summary comments ✓
- **Edge case**: if `author` is empty string or null, would be included — verify this is handled
- No bug found, but add a guard for empty author

**Attachment path in context file**:
- Must be updated after filename collision fix (see §2 above)

#### 6. `backend/src/ai/litellm.rs` — `complete_json()` Response Parsing

**Token count extraction**:
- Current: extracts `usage.prompt_tokens` and `usage.completion_tokens` from response
- Verify field names match LiteLLM's actual response format (OpenAI-compat uses `prompt_tokens`/`completion_tokens`) ✓
- **Edge case**: if `choices` is empty or `message.content` is null, code should return a clear error rather than panic
- **Fix**: add explicit error for empty choices array and null content

---

## Sub-plan 2: Backend Test Coverage to 90%

### Current State

320+ tests across 18 test files. Zero coverage for:
- `src/ai/context_manager.rs` (424 lines)
- `src/ai/litellm.rs` (172 lines)
- `src/api/attachments.rs` (155 lines)
- `src/db/attachments.rs` (60 lines)
- Pipeline integration (full path from attachments + comments → context file)

### New Test Files

#### `tests/attachment_db_test.rs`
Tests for `AttachmentRepository`:
- `create` stores attachment and returns it with correct fields
- `list_for_task` returns attachments in insertion order
- `list_for_task` returns empty vec for task with no attachments
- `get` returns Some for existing, None for unknown
- `delete` removes record; subsequent `get` returns None
- Cascade: deleting a task (via `TaskRepository`) removes its attachments

#### `tests/attachment_api_test.rs`
Tests for attachment HTTP handlers via `TestServer`:
- `GET /api/tasks/:id/attachments` returns empty list for new task
- `POST /api/tasks/:id/attachments` with valid image returns 200 + TaskAttachment JSON
- `POST` with unknown task_id returns 404
- `POST` with no file field returns 400
- `GET /api/tasks/:id/attachments` after upload returns list with one item
- `DELETE /api/tasks/:id/attachments/:att_id` returns 204; subsequent GET shows empty list
- `DELETE` with wrong task_id returns 404
- `GET /api/tasks/:id/attachments/:att_id/file` serves file bytes with correct Content-Type
- Filename sanitization: upload file with `../../../etc/passwd` name, verify stored name is safe

#### `tests/litellm_test.rs`
Tests for `LitellmClient` and helpers:
- `image_to_data_url` with PNG file returns `data:image/png;base64,...`
- `image_to_data_url` with JPEG file and explicit mime returns `data:image/jpeg;base64,...`
- `image_to_data_url` with nonexistent path returns `None`
- `build_user_message` with no images returns `{"role":"user","content":"<text>"}`
- `build_user_message` with images returns content array with text part + image_url parts
- `complete_json` with mock HTTP server: parses content, input/output tokens, latency
- `complete_json` with empty choices returns `Err`
- `complete_json` with HTTP 500 returns `Err`

#### `tests/context_manager_test.rs`
Tests for `ContextManager` with a mock HTTP server (using `wiremock` or `mockito`):
- `summarize_session` with empty display_lines skips LiteLLM call (returns Ok)
- `summarize_session` posts summary as "litellm" comment on task
- `summarize_session` includes image_url parts when task has image attachments
- `enrich_task` with empty LiteLLM response returns `Ok(None)` and logs warning
- `enrich_task` stores enriched text in `task.instructions`
- `compress_context` stores compressed text in `task.compressed_context`
- `generate_briefing` returns formatted string with LiteLLM content

#### `tests/pipeline_test.rs`
End-to-end backend pipeline tests (real DB, no Claude binary):
- Create task + upload 2 attachments (1 image, 1 PDF) → call `write_task_context_file` → verify context file has `## Attached Files` section with both files
- Verify image attachment uses `{id}-{filename}` path in context file
- Create task + add 3 comments (2 user, 1 litellm) → call `write_task_context_file` → verify litellm comment excluded from `## Discussion`
- Create task with `instructions` → verify `## Implementation Plan` section present in context file
- Create task with `compressed_context` → verify `## Prior Session Context` section present
- Full pipeline: task + image attachment → `task_image_data_urls()` returns base64 data URL with correct MIME

### Coverage Target

Use `cargo-llvm-cov` or `cargo tarpaulin` to measure. Target: ≥90% line coverage across `src/`.

---

## Sub-plan 3: Frontend Vitest Setup and Unit Test Coverage to 90%

### Infrastructure Changes

#### `package.json` devDependencies to add:
```json
"vitest": "^2.0.0",
"@vitest/coverage-v8": "^2.0.0",
"@testing-library/react": "^16.0.0",
"@testing-library/jest-dom": "^6.0.0",
"@testing-library/user-event": "^14.0.0",
"jsdom": "^25.0.0",
"msw": "^2.0.0"
```

#### `vitest.config.ts`:
```typescript
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      thresholds: { lines: 90, functions: 90, branches: 85, statements: 90 },
      include: ['src/**/*.{ts,tsx}'],
      exclude: ['src/test/**', 'src/**/*.d.ts', 'src/app/layout.tsx'],
    },
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
```

#### `package.json` scripts to add:
```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage",
"test:ui": "vitest --ui"
```

### Components/Hooks Currently Lacking Tests

| File | Status |
|------|--------|
| `components/kanban/kanban-card.tsx` | No tests |
| `components/kanban/kanban-column.tsx` | No tests |
| `components/kanban/kanban-board.tsx` | No tests |
| `components/tasks/comment-input.tsx` | No tests |
| `components/tasks/comment-thread.tsx` | No tests |
| `components/tasks/task-detail.tsx` | No tests |
| `components/layout/sidebar.tsx` | No tests |
| `hooks/use-comments.ts` | No tests |
| `hooks/use-settings.ts` | No tests |
| `hooks/use-task-subscriptions.ts` | No tests |
| `contexts/websocket-context.tsx` | No tests |
| `lib/pricing.ts` | No tests |

### New Test Files

#### `src/test/components/kanban-card.test.tsx`
- Renders task title and stage badge
- Shows cost chip when costData provided
- Shows token chip with formatted value
- Does not render chips when no costData

#### `src/test/components/kanban-column.test.tsx`
- Renders column header with stage name and task count
- Renders KanbanCard for each task
- Shows empty state when no tasks

#### `src/test/components/comment-input.test.tsx`
- Renders textarea and submit button
- Submit disabled when content empty
- Calls createComment mutation on submit
- Clears content after successful submit
- Shows pending image thumbnails on paste
- Remove button revokes object URL

#### `src/test/components/comment-thread.test.tsx`
- Renders comment author and content
- Renders markdown in comment body
- Renders inline images via `![alt](url)` markdown
- Shows reply input on reply button click
- Renders nested replies

#### `src/test/components/sidebar.test.tsx`
- Renders navigation links
- Renders SidebarMetrics section
- Shows active state for current route

#### `src/test/hooks/use-comments.test.ts`
- `useComments` fetches comment list for task
- `useCreateComment` posts and invalidates cache
- `useDeleteComment` deletes and invalidates cache

#### `src/test/hooks/use-settings.test.ts`
- `useSettings` returns settings object
- `useUpdateSetting` patches and invalidates

#### `src/test/lib/pricing.test.ts`
- Cost calculation for known token counts
- Returns zero for zero tokens
- Handles undefined model gracefully

### Coverage Target

≥90% line/function coverage measured via `vitest run --coverage`. Branches ≥85%.

---

## Architecture Notes

- **Test isolation**: every backend test uses a unique `/tmp/test-{uuid}.db` path — no shared state between tests
- **Mock HTTP for LiteLLM**: `context_manager_test.rs` and `litellm_test.rs` use a local mock HTTP server (wiremock crate) to avoid real LiteLLM calls in CI
- **Frontend MSW**: existing `src/test/msw/` infrastructure (server.ts, handlers.ts, fixtures.ts) is already wired — new tests use it for API mocking
- **No snapshot tests**: snapshots are brittle for this codebase; prefer explicit assertions

## File Map

**Bug fixes touch:**
- `backend/src/api/attachments.rs`
- `backend/src/claude/manager.rs`
- `backend/src/ai/litellm.rs`
- `backend/src/ai/context_manager.rs`

**New backend test files:**
- `backend/tests/attachment_db_test.rs`
- `backend/tests/attachment_api_test.rs`
- `backend/tests/litellm_test.rs`
- `backend/tests/context_manager_test.rs`
- `backend/tests/pipeline_test.rs`

**Frontend infrastructure:**
- `frontend/package.json`
- `frontend/vitest.config.ts`

**New frontend test files:**
- `frontend/src/test/components/kanban-card.test.tsx`
- `frontend/src/test/components/kanban-column.test.tsx`
- `frontend/src/test/components/comment-input.test.tsx`
- `frontend/src/test/components/comment-thread.test.tsx`
- `frontend/src/test/components/sidebar.test.tsx`
- `frontend/src/test/hooks/use-comments.test.ts`
- `frontend/src/test/hooks/use-settings.test.ts`
- `frontend/src/test/lib/pricing.test.ts`
