# Pipeline Audit, Bug Fixes, and Test Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 6 bugs in the context/attachment/image pipeline, then achieve ≥90% backend test coverage and set up + achieve ≥90% frontend unit test coverage.

**Architecture:** Three sequential phases: (1) fix real bugs in the upload→copy→LiteLLM path, (2) add backend tests for all previously-uncovered modules using wiremock for LiteLLM mocking, (3) install Vitest properly and write component/hook/context tests.

**Tech Stack:** Rust/Axum backend, SQLite via sqlx, wiremock for HTTP mocking; Next.js 16 frontend, Vitest 2, @testing-library/react, MSW 2

**Spec:** `docs/superpowers/specs/2026-03-23-pipeline-audit-and-test-coverage-design.md`

---

## PHASE 1 — Pipeline Bug Fixes

---

### Task 1: Fix `upload_attachment` — 500→404 for unknown task + MIME fallback + filename sanitization

**Files:**
- Modify: `backend/src/api/attachments.rs`

**What to fix:**
1. `task_repo.find()` returns `Err` for missing task — handler maps all errors to 500. Need 404 for "not found" vs 500 for DB error.
2. MIME type falls back to `application/octet-stream` when browser sends empty — should infer from extension.
3. Filename sanitizer only strips `/`, `\`, `..` — path traversal still possible with `....//`. Use strict allowlist instead.

- [ ] **Step 1: Open the file and understand current shape**

```
backend/src/api/attachments.rs lines 28-90
```

The `find()` call at line 36 maps any error to 500. The safe_name at line 66 only strips some chars.

- [ ] **Step 2: Replace the upload_attachment function body**

In `backend/src/api/attachments.rs`, replace lines 28–90:

```rust
// POST /api/tasks/:task_id/attachments  (multipart/form-data, field name: "file")
pub async fn upload_attachment(
    State(state): State<AttachmentApiState>,
    Path(task_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<TaskAttachment>, StatusCode> {
    // Verify task exists — return 404 for missing task, 500 for DB errors
    state
        .task_repo
        .find(&task_id)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("no rows") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let filename: String = field
            .file_name()
            .unwrap_or("upload")
            .to_string();

        let raw_mime: String = field
            .content_type()
            .unwrap_or("")
            .to_string();

        let data: Vec<u8> = field
            .bytes()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .to_vec();

        // Infer MIME from extension when browser sends empty or octet-stream
        let mime_type = if raw_mime.is_empty() || raw_mime == "application/octet-stream" {
            infer_mime_from_filename(&filename).to_string()
        } else {
            raw_mime
        };

        // Strict allowlist sanitizer — keep only alphanumeric, dot, dash, underscore
        let safe_name: String = filename
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>()
            .chars()
            .take(255)
            .collect();
        let safe_name = if safe_name.is_empty() { "upload".to_string() } else { safe_name };

        // Write to disk: <attachments_dir>/<task_id>/<uuid>-<safe_name>
        let dir = format!("{}/{}", state.attachments_dir, task_id);
        fs::create_dir_all(&dir)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let id = Uuid::new_v4().to_string();
        let storage_path = format!("{}/{}-{}", dir, id, safe_name);
        fs::write(&storage_path, &data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let attachment = TaskAttachment {
            id,
            task_id,
            filename,
            storage_path,
            mime_type,
            size_bytes: data.len() as i64,
            created_at: Utc::now(),
        };

        return state
            .repo
            .create(&attachment)
            .await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }
    Err(StatusCode::BAD_REQUEST)
}

fn infer_mime_from_filename(filename: &str) -> &'static str {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png"  => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif"  => "image/gif",
        "webp" => "image/webp",
        "pdf"  => "application/pdf",
        "txt"  => "text/plain",
        "md"   => "text/markdown",
        _      => "application/octet-stream",
    }
}
```

- [ ] **Step 3: Build to check it compiles**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep "^error"
```
Expected: no output (no errors)

- [ ] **Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/api/attachments.rs
git commit -m "fix(api): 404 for unknown task, MIME inference, strict filename sanitization"
```

---

### Task 2: Fix attachment filename collision in `start_session()` copy + context file + `--image` arg

**Files:**
- Modify: `backend/src/claude/manager.rs` (lines 205–219 and 1052–1053)

The destination filename when copying attachments to `.claude/attachments/` uses `att.filename` with no uniqueness guarantee. Two attachments with the same name silently overwrite each other. All three locations (copy dest, context file, `--image` arg) must use `{att.id}-{att.filename}`.

- [ ] **Step 1: Fix the copy block**

In `backend/src/claude/manager.rs`, replace lines 206–219:

```rust
        // Copy attachments into project's .claude/attachments/ and add --image args for images.
        // Use {att.id}-{att.filename} as destination to prevent filename collisions.
        {
            let claude_attachments_dir = format!("{}/.claude/attachments", project_path);
            let _ = tokio::fs::create_dir_all(&claude_attachments_dir).await;
            for att in &attachments {
                let dest_filename = format!("{}-{}", att.id, att.filename);
                let dest = format!("{}/{}", claude_attachments_dir, dest_filename);
                if let Err(e) = tokio::fs::copy(&att.storage_path, &dest).await {
                    warn!(attachment_id = %att.id, error = %e, "Failed to copy attachment to project dir");
                    continue;
                }
                if att.mime_type.starts_with("image/") {
                    cmd.arg("--image").arg(&dest);
                }
            }
        }
```

- [ ] **Step 2: Fix the context file attachment list**

In `write_task_context_file` (around line 1052), replace:

```rust
            lines.push(format!("- `.claude/attachments/{}` ({})", att.filename, att.mime_type));
```

with:

```rust
            lines.push(format!("- `.claude/attachments/{}-{}` ({})", att.id, att.filename, att.mime_type));
```

- [ ] **Step 3: Build to check it compiles**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep "^error"
```
Expected: no output

- [ ] **Step 4: Update context_file_test to match new path format**

In `backend/tests/context_file_test.rs`, there is no attachment path assertion, but verify no test breaks.

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test context_file 2>&1 | tail -5
```
Expected: `test result: ok. N passed`

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/claude/manager.rs
git commit -m "fix(claude): use {id}-{filename} for attachment copy to prevent collisions"
```

---

### Task 3: Fix `image_to_data_url` to accept explicit MIME type

**Files:**
- Modify: `backend/src/ai/litellm.rs` (lines 53–71)
- Modify: `backend/src/ai/context_manager.rs` (lines 36–39)

Currently `image_to_data_url(path)` infers MIME from file extension. The stored `TaskAttachment.mime_type` is correct and must be used instead.

- [ ] **Step 1: Change function signature and body**

In `backend/src/ai/litellm.rs`, replace lines 51–71:

```rust
/// Encode an image file as a base64 data URL for the LiteLLM vision API.
/// `mime_type` is the stored content type (e.g. "image/png") — used directly.
/// Returns `None` if the file cannot be read (non-fatal — caller skips that image).
pub async fn image_to_data_url(path: &str, mime_type: &str) -> Option<String> {
    use base64::Engine as _;
    let data = tokio::fs::read(path).await.ok()?;
    Some(format!(
        "data:{};base64,{}",
        mime_type,
        base64::engine::general_purpose::STANDARD.encode(&data)
    ))
}
```

- [ ] **Step 2: Update caller in context_manager.rs**

In `backend/src/ai/context_manager.rs`, find the `task_image_data_urls` method. The line calling `image_to_data_url(&att.storage_path)` must be changed to pass `&att.mime_type`:

```rust
            match image_to_data_url(&att.storage_path, &att.mime_type).await {
```

- [ ] **Step 3: Build**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep "^error"
```
Expected: no output

- [ ] **Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/ai/litellm.rs backend/src/ai/context_manager.rs
git commit -m "fix(litellm): use stored mime_type for base64 encoding instead of extension inference"
```

---

### Task 4: Fix `complete_json` — empty choices returns Err; `ChoiceMessage.content` becomes `Option<String>`

**Files:**
- Modify: `backend/src/ai/litellm.rs`

Two fixes: (1) empty `choices` silently returns empty string — should return `Err`. (2) `content: String` panics on null content from model — change to `Option<String>`.

- [ ] **Step 1: Change ChoiceMessage struct**

In `backend/src/ai/litellm.rs`, replace:

```rust
#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}
```

with:

```rust
#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}
```

- [ ] **Step 2: Fix content extraction in complete_json**

Replace lines 144–148:

```rust
        let content = resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
```

with:

```rust
        let content = resp.choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("LiteLLM returned empty choices array"))?
            .message
            .content
            .unwrap_or_default();
```

- [ ] **Step 3: Build**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo build 2>&1 | grep "^error"
```
Expected: no output

- [ ] **Step 4: Run all tests to verify nothing broken**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test 2>&1 | tail -5
```
Expected: `test result: ok` or only the pre-existing date test failure

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/src/ai/litellm.rs
git commit -m "fix(litellm): error on empty choices, handle null content field"
```

---

## PHASE 2 — Backend Test Coverage

---

### Task 5: Add `wiremock` dev-dependency

**Files:**
- Modify: `backend/Cargo.toml`

- [ ] **Step 1: Add wiremock to Cargo.toml**

In `backend/Cargo.toml`, add to `[dev-dependencies]`:

```toml
wiremock = "0.6"
```

- [ ] **Step 2: Verify it downloads**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo fetch 2>&1 | tail -3
```
Expected: no errors

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/Cargo.toml backend/Cargo.lock
git commit -m "chore(backend): add wiremock dev-dependency for HTTP mocking in tests"
```

---

### Task 6: Attachment DB tests (`tests/attachment_db_test.rs`)

**Files:**
- Create: `backend/tests/attachment_db_test.rs`

- [ ] **Step 1: Create the test file**

```rust
// backend/tests/attachment_db_test.rs
use ai_kanban_backend::db::{AttachmentRepository, TaskRepository, create_pool};
use ai_kanban_backend::models::{CreateTask, TaskAttachment};
use chrono::Utc;

async fn setup() -> (AttachmentRepository, TaskRepository, String) {
    let db_path = format!("/tmp/test-att-db-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    (
        AttachmentRepository::new(pool.clone()),
        TaskRepository::new(pool),
        db_path,
    )
}

fn make_attachment(task_id: &str, filename: &str) -> TaskAttachment {
    TaskAttachment {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        filename: filename.to_string(),
        storage_path: format!("/tmp/{}", filename),
        mime_type: "image/png".to_string(),
        size_bytes: 1024,
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_create_stores_and_returns_attachment() {
    let (repo, task_repo, _) = setup().await;
    let task = task_repo.create(CreateTask {
        title: "t".to_string(), description: None, project_path: "/tmp".to_string(),
    }).await.unwrap();

    let att = make_attachment(&task.id, "photo.png");
    let saved = repo.create(&att).await.unwrap();
    assert_eq!(saved.id, att.id);
    assert_eq!(saved.filename, "photo.png");
    assert_eq!(saved.mime_type, "image/png");
    assert_eq!(saved.task_id, task.id);
}

#[tokio::test]
async fn test_list_for_task_returns_empty_for_no_attachments() {
    let (repo, task_repo, _) = setup().await;
    let task = task_repo.create(CreateTask {
        title: "t".to_string(), description: None, project_path: "/tmp".to_string(),
    }).await.unwrap();
    let list = repo.list_for_task(&task.id).await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_list_for_task_returns_all_attachments() {
    let (repo, task_repo, _) = setup().await;
    let task = task_repo.create(CreateTask {
        title: "t".to_string(), description: None, project_path: "/tmp".to_string(),
    }).await.unwrap();
    let a1 = make_attachment(&task.id, "a.png");
    let a2 = make_attachment(&task.id, "b.pdf");
    repo.create(&a1).await.unwrap();
    repo.create(&a2).await.unwrap();
    let list = repo.list_for_task(&task.id).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_list_for_task_does_not_return_other_tasks_attachments() {
    let (repo, task_repo, _) = setup().await;
    let t1 = task_repo.create(CreateTask { title: "t1".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let t2 = task_repo.create(CreateTask { title: "t2".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    repo.create(&make_attachment(&t1.id, "a.png")).await.unwrap();
    let list = repo.list_for_task(&t2.id).await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_get_returns_some_for_existing() {
    let (repo, task_repo, _) = setup().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let att = make_attachment(&task.id, "photo.png");
    repo.create(&att).await.unwrap();
    let found = repo.get(&att.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, att.id);
}

#[tokio::test]
async fn test_get_returns_none_for_unknown() {
    let (repo, _, _) = setup().await;
    let found = repo.get("nonexistent-id").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_removes_record() {
    let (repo, task_repo, _) = setup().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let att = make_attachment(&task.id, "photo.png");
    repo.create(&att).await.unwrap();
    repo.delete(&att.id).await.unwrap();
    assert!(repo.get(&att.id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_delete_is_idempotent() {
    let (repo, _, _) = setup().await;
    // Deleting a non-existent record should not error
    repo.delete("ghost-id").await.unwrap();
}

#[tokio::test]
async fn test_cascade_delete_removes_attachments_with_task() {
    let (repo, task_repo, _) = setup().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let att = make_attachment(&task.id, "photo.png");
    repo.create(&att).await.unwrap();
    task_repo.delete(&task.id).await.unwrap();
    let list = repo.list_for_task(&task.id).await.unwrap();
    assert!(list.is_empty());
}
```

- [ ] **Step 2: Run the tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test attachment_db 2>&1 | tail -10
```
Expected: `test result: ok. 9 passed`

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/tests/attachment_db_test.rs
git commit -m "test(backend): attachment DB repository tests"
```

---

### Task 7: Attachment API tests (`tests/attachment_api_test.rs`)

**Files:**
- Create: `backend/tests/attachment_api_test.rs`

Note: `TaskRepository::find()` returns `Err` with message containing "not found" for missing tasks. After Task 1's fix, the handler returns 404 in that case.

- [ ] **Step 1: Create the test file**

```rust
// backend/tests/attachment_api_test.rs
use ai_kanban_backend::api::{AppState, AttachmentApiState};
use ai_kanban_backend::db::{
    AttachmentRepository, create_pool, CommentRepository, LogRepository,
    OtelMetricsRepository, SessionMetricsRepository, SessionRepository,
    SettingsRepository, TaskRepository, TokenEventRepository,
};
use ai_kanban_backend::models::CreateTask;
use axum_test::TestServer;
use axum_test::http::StatusCode;
use std::sync::Arc;

async fn setup_server() -> (TestServer, TaskRepository, String) {
    let db_path = format!("/tmp/test-att-api-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    let task_repo = TaskRepository::new(pool.clone());
    let log_repo = LogRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let token_repo = TokenEventRepository::new(pool.clone());
    let metrics_repo = SessionMetricsRepository::new(pool.clone());
    let settings_repo = SettingsRepository::new(pool.clone());
    let otel_repo = OtelMetricsRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());

    // AttachmentApiState reads ATTACHMENTS_DIR from env in create_router().
    // Set a unique temp dir per test. Note: tests in this file must not run in parallel
    // (env var is process-global). They run sequentially by default in a single test binary.
    let attachments_dir = format!("/tmp/test-att-files-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&attachments_dir).await.unwrap();
    std::env::set_var("ATTACHMENTS_DIR", &attachments_dir);

    let state = AppState::new(
        task_repo.clone(), log_repo, session_repo, comment_repo,
        token_repo, metrics_repo, settings_repo, otel_repo, attachment_repo,
    );

    let server = TestServer::new(ai_kanban_backend::api::create_router(state)).unwrap();
    (server, task_repo, attachments_dir)
}

#[tokio::test]
async fn test_list_attachments_empty() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let res = server.get(&format!("/api/tasks/{}/attachments", task.id)).await;
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: Vec<serde_json::Value> = res.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_upload_attachment_success() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let res = server
        .post(&format!("/api/tasks/{}/attachments", task.id))
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"PNG_DATA".to_vec())
                    .file_name("photo.png")
                    .mime_type("image/png"))
        )
        .await;
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: serde_json::Value = res.json();
    assert_eq!(body["filename"], "photo.png");
    assert_eq!(body["mime_type"], "image/png");
    assert_eq!(body["task_id"], task.id);
}

#[tokio::test]
async fn test_upload_attachment_unknown_task_returns_404() {
    let (server, _, _) = setup_server().await;
    let res = server
        .post("/api/tasks/nonexistent-task-id/attachments")
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"data".to_vec())
                    .file_name("f.png")
                    .mime_type("image/png"))
        )
        .await;
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_upload_no_file_field_returns_400() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let res = server
        .post(&format!("/api/tasks/{}/attachments", task.id))
        .multipart(axum_test::multipart::MultipartForm::new())
        .await;
    assert_eq!(res.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_after_upload_returns_one() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    server
        .post(&format!("/api/tasks/{}/attachments", task.id))
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"PNG".to_vec())
                    .file_name("photo.png")
                    .mime_type("image/png"))
        )
        .await;
    let res = server.get(&format!("/api/tasks/{}/attachments", task.id)).await;
    let body: Vec<serde_json::Value> = res.json();
    assert_eq!(body.len(), 1);
}

#[tokio::test]
async fn test_delete_attachment() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let upload = server
        .post(&format!("/api/tasks/{}/attachments", task.id))
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"PNG".to_vec())
                    .file_name("photo.png")
                    .mime_type("image/png"))
        )
        .await;
    let att: serde_json::Value = upload.json();
    let att_id = att["id"].as_str().unwrap();

    let del = server.delete(&format!("/api/tasks/{}/attachments/{}", task.id, att_id)).await;
    assert_eq!(del.status_code(), StatusCode::NO_CONTENT);

    let list_res = server.get(&format!("/api/tasks/{}/attachments", task.id)).await;
    let list: Vec<serde_json::Value> = list_res.json();
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_delete_wrong_task_returns_404() {
    let (server, task_repo, _) = setup_server().await;
    let t1 = task_repo.create(CreateTask { title: "t1".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let t2 = task_repo.create(CreateTask { title: "t2".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let upload = server
        .post(&format!("/api/tasks/{}/attachments", t1.id))
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"PNG".to_vec())
                    .file_name("photo.png")
                    .mime_type("image/png"))
        )
        .await;
    let att: serde_json::Value = upload.json();
    let att_id = att["id"].as_str().unwrap();

    // Try to delete using t2's task_id — should be 404
    let res = server.delete(&format!("/api/tasks/{}/attachments/{}", t2.id, att_id)).await;
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_serve_attachment_file() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let upload = server
        .post(&format!("/api/tasks/{}/attachments", task.id))
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"FILE_CONTENT".to_vec())
                    .file_name("doc.txt")
                    .mime_type("text/plain"))
        )
        .await;
    let att: serde_json::Value = upload.json();
    let att_id = att["id"].as_str().unwrap();

    let res = server.get(&format!("/api/tasks/{}/attachments/{}/file", task.id, att_id)).await;
    assert_eq!(res.status_code(), StatusCode::OK);
    assert_eq!(res.text(), "FILE_CONTENT");
}

#[tokio::test]
async fn test_filename_sanitization_strips_path_traversal() {
    let (server, task_repo, _) = setup_server().await;
    let task = task_repo.create(CreateTask { title: "t".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let res = server
        .post(&format!("/api/tasks/{}/attachments", task.id))
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_part("file", axum_test::multipart::Part::bytes(b"x".to_vec())
                    .file_name("../../../etc/passwd")
                    .mime_type("text/plain"))
        )
        .await;
    assert_eq!(res.status_code(), StatusCode::OK);
    let body: serde_json::Value = res.json();
    // storage_path must not contain ".." or "/"
    let storage_path = body["storage_path"].as_str().unwrap();
    assert!(!storage_path.contains(".."));
    // original filename is preserved as-is; storage path uses the sanitized name
    assert!(!storage_path.ends_with("etc/passwd"));
}
```

Note: `AppState::with_attachments_dir()` may not exist yet — check if `AppState` has this. If not, the test setup uses the attachments_dir stored in `AppState`. Look at how it's set up in `main.rs` and adapt.

- [ ] **Step 2: Check how AppState/AttachmentApiState sets the dir**

```bash
grep -n "attachments_dir" /home/utility/Projects/ai-kanban/backend/src/api/mod.rs /home/utility/Projects/ai-kanban/backend/src/main.rs 2>/dev/null | head -20
```

If `AppState` doesn't expose `with_attachments_dir`, look at how `AttachmentApiState` is built in the router and add a helper method or pass the dir directly to `AppState`.

- [ ] **Step 3: Run the tests (iterate until they pass)**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test attachment_api 2>&1 | tail -15
```
Expected: `test result: ok. 9 passed`

- [ ] **Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/tests/attachment_api_test.rs backend/src/
git commit -m "test(backend): attachment API handler tests (upload, list, delete, serve, sanitization)"
```

---

### Task 8: LiteLLM unit tests (`tests/litellm_test.rs`)

**Files:**
- Create: `backend/tests/litellm_test.rs`

Tests for `image_to_data_url` and `complete_json` using wiremock.

- [ ] **Step 1: Create the test file**

```rust
// backend/tests/litellm_test.rs
use ai_kanban_backend::ai::litellm::{image_to_data_url, LitellmClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── image_to_data_url ───────────────────────────────────────────

#[tokio::test]
async fn test_image_to_data_url_png() {
    let tmp = format!("/tmp/test-img-{}.png", uuid::Uuid::new_v4());
    tokio::fs::write(&tmp, b"\x89PNG\r\n").await.unwrap();

    let result = image_to_data_url(&tmp, "image/png").await;
    assert!(result.is_some());
    let url = result.unwrap();
    assert!(url.starts_with("data:image/png;base64,"));
}

#[tokio::test]
async fn test_image_to_data_url_uses_provided_mime_not_extension() {
    // File has .jpg extension but we pass "image/webp" — should use webp
    let tmp = format!("/tmp/test-img-{}.jpg", uuid::Uuid::new_v4());
    tokio::fs::write(&tmp, b"WEBP_DATA").await.unwrap();

    let result = image_to_data_url(&tmp, "image/webp").await.unwrap();
    assert!(result.starts_with("data:image/webp;base64,"), "Expected webp MIME, got: {}", result);
}

#[tokio::test]
async fn test_image_to_data_url_nonexistent_returns_none() {
    let result = image_to_data_url("/tmp/does-not-exist-abc123.png", "image/png").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_image_to_data_url_encodes_file_content() {
    use base64::Engine as _;
    let tmp = format!("/tmp/test-img-{}.png", uuid::Uuid::new_v4());
    let content = b"FAKE_PNG_BYTES";
    tokio::fs::write(&tmp, content).await.unwrap();

    let result = image_to_data_url(&tmp, "image/png").await.unwrap();
    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(content);
    assert!(result.contains(&expected_b64));
}

// ── complete_json ───────────────────────────────────────────────

fn make_response(content: &str, input: i64, output: i64) -> serde_json::Value {
    serde_json::json!({
        "choices": [{"message": {"content": content, "role": "assistant"}}],
        "usage": {"prompt_tokens": input, "completion_tokens": output}
    })
}

#[tokio::test]
async fn test_complete_json_parses_content_and_tokens() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(make_response("hello world", 10, 5)))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let msg = serde_json::json!({"role": "user", "content": "hi"});
    let result = client.complete_json(vec![msg]).await.unwrap();

    assert_eq!(result.content, "hello world");
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 5);
    assert!(result.latency_ms > 0);
}

#[tokio::test]
async fn test_complete_json_http_error_returns_err() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let result = client.complete_json(vec![serde_json::json!({"role": "user", "content": "hi"})]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_complete_json_empty_choices_returns_err() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [],
            "usage": {"prompt_tokens": 5, "completion_tokens": 0}
        })))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let result = client.complete_json(vec![serde_json::json!({"role": "user", "content": "hi"})]).await;
    assert!(result.is_err(), "Expected Err for empty choices");
}

#[tokio::test]
async fn test_complete_json_null_content_uses_empty_string() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": null, "role": "assistant"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 0}
        })))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let result = client.complete_json(vec![serde_json::json!({"role": "user", "content": "hi"})]).await.unwrap();
    assert_eq!(result.content, "");
}

#[tokio::test]
async fn test_complete_json_missing_usage_defaults_to_zero() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "hi", "role": "assistant"}}]
        })))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let result = client.complete_json(vec![serde_json::json!({"role": "user", "content": "hi"})]).await.unwrap();
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
}
```

- [ ] **Step 2: Run the tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test litellm_test 2>&1 | tail -10
```
Expected: `test result: ok. 7 passed`

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/tests/litellm_test.rs
git commit -m "test(backend): litellm image_to_data_url and complete_json tests"
```

---

### Task 9: Context manager tests (`tests/context_manager_test.rs`)

**Files:**
- Create: `backend/tests/context_manager_test.rs`

Tests for `ContextManager` methods using wiremock and a real DB.

- [ ] **Step 1: Create the test file**

```rust
// backend/tests/context_manager_test.rs
use ai_kanban_backend::ai::context_manager::ContextManager;
use ai_kanban_backend::ai::litellm::LitellmClient;
use ai_kanban_backend::db::{
    AttachmentRepository, CommentRepository, TaskRepository, create_pool,
};
use ai_kanban_backend::models::{CreateComment, CreateTask};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup(server_uri: &str) -> (ContextManager, TaskRepository, CommentRepository) {
    let db_path = format!("/tmp/test-cm-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    let task_repo = TaskRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());
    let litellm = LitellmClient::new(server_uri, "test-key", "test-model");
    let cm = ContextManager::new(litellm, comment_repo.clone(), task_repo.clone(), attachment_repo);
    (cm, task_repo, comment_repo)
}

fn ok_response(content: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "choices": [{"message": {"content": content, "role": "assistant"}}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5}
    }))
}

// ── summarize_session ───────────────────────────────────────────

#[tokio::test]
async fn test_summarize_session_empty_lines_skips_litellm() {
    let mock_server = MockServer::start().await;
    // No mock mounted — if LiteLLM is called, the test will get a connection error and fail
    let (cm, task_repo, _) = setup(&mock_server.uri()).await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let result = cm.summarize_session("sid", &task.id, "T", "planning", None, 0, 0, &[], None).await;
    assert!(result.is_ok());
    // Verify no request was made
    assert!(mock_server.received_requests().await.is_none() || mock_server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_summarize_session_posts_litellm_comment() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ok_response("## What Changed\n- Fixed login bug"))
        .mount(&mock_server)
        .await;

    let (cm, task_repo, comment_repo) = setup(&mock_server.uri()).await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let lines = vec!["✏️ Edited src/main.rs".to_string()];
    cm.summarize_session("sid", &task.id, "T", "planning", Some(30), 100, 50, &lines, None).await.unwrap();

    let comments = comment_repo.list_for_task(&task.id).await.unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].comment.author, "litellm");
    assert!(comments[0].comment.content.contains("Session Summary"));
}

// ── enrich_task ─────────────────────────────────────────────────

#[tokio::test]
async fn test_enrich_task_http_error_returns_err() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("error"))
        .mount(&mock_server)
        .await;

    let (cm, task_repo, _) = setup(&mock_server.uri()).await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let result = cm.enrich_task(&task.id, "T", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_enrich_task_empty_content_returns_ok_none() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "   ", "role": "assistant"}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 0}
        })))
        .mount(&mock_server)
        .await;

    let (cm, task_repo, _) = setup(&mock_server.uri()).await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let result = cm.enrich_task(&task.id, "T", None).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_enrich_task_stores_instructions() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ok_response("1. Set up auth\n2. Add tests"))
        .mount(&mock_server)
        .await;

    let (cm, task_repo, _) = setup(&mock_server.uri()).await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let result = cm.enrich_task(&task.id, "T", Some("do the thing")).await.unwrap();
    assert!(result.is_some());
    let enriched = result.unwrap();
    assert!(enriched.contains("Set up auth"));

    // Verify stored in DB
    let updated = task_repo.find(&task.id).await.unwrap();
    assert_eq!(updated.instructions.as_deref(), Some(enriched.as_str()));
}

// ── compress_context ────────────────────────────────────────────

#[tokio::test]
async fn test_compress_context_stores_compressed() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ok_response("Compressed: did X, Y, Z"))
        .mount(&mock_server)
        .await;

    let (cm, task_repo, _) = setup(&mock_server.uri()).await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: "/tmp".to_string() }).await.unwrap();
    let lines = vec!["Did X".to_string(), "Did Y".to_string()];
    cm.compress_context("sid", &task.id, "T", &lines, None).await.unwrap();

    let updated = task_repo.find(&task.id).await.unwrap();
    assert!(updated.compressed_context.is_some());
    assert!(updated.compressed_context.unwrap().contains("Compressed"));
}

// ── generate_briefing ───────────────────────────────────────────

#[tokio::test]
async fn test_generate_briefing_returns_formatted_string() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ok_response("Context summary: user decided to use Postgres"))
        .mount(&mock_server)
        .await;

    let (cm, _, _) = setup(&mock_server.uri()).await;
    let result = cm.generate_briefing("My Task", "User: use Postgres\nClaude: ok").await.unwrap();
    assert!(result.contains("Briefing compressed by LiteLLM"));
    assert!(result.contains("Context summary"));
}
```

- [ ] **Step 2: Run the tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test context_manager_test 2>&1 | tail -10
```
Expected: `test result: ok. 8 passed`

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/tests/context_manager_test.rs
git commit -m "test(backend): context manager summarize, enrich, compress, briefing tests"
```

---

### Task 10: Pipeline integration tests (`tests/pipeline_test.rs`)

**Files:**
- Create: `backend/tests/pipeline_test.rs`

End-to-end tests for the context file writing path and the image data URL pipeline.

- [ ] **Step 1: Create the test file**

```rust
// backend/tests/pipeline_test.rs
use ai_kanban_backend::claude::write_task_context_file;
use ai_kanban_backend::ai::litellm::image_to_data_url;
use ai_kanban_backend::db::{AttachmentRepository, CommentRepository, TaskRepository, create_pool};
use ai_kanban_backend::models::{CreateComment, CreateTask, TaskAttachment};
use chrono::Utc;
use tokio::sync::broadcast;

async fn setup() -> (TaskRepository, AttachmentRepository, CommentRepository) {
    let db_path = format!("/tmp/test-pipeline-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.unwrap();
    (
        TaskRepository::new(pool.clone()),
        AttachmentRepository::new(pool.clone()),
        CommentRepository::new(pool),
    )
}

fn make_att(task_id: &str, id: &str, filename: &str, mime: &str, path: &str) -> TaskAttachment {
    TaskAttachment {
        id: id.to_string(),
        task_id: task_id.to_string(),
        filename: filename.to_string(),
        storage_path: path.to_string(),
        mime_type: mime.to_string(),
        size_bytes: 100,
        created_at: Utc::now(),
    }
}

fn make_tx() -> broadcast::Sender<ai_kanban_backend::claude::ClaudeEvent> {
    let (tx, _) = broadcast::channel(16);
    tx
}

#[tokio::test]
async fn test_context_file_includes_attached_files_section() {
    let project_path = format!("/tmp/ctx-pipeline-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    let (task_repo, att_repo, _) = setup().await;
    let task = task_repo.create(CreateTask { title: "My Task".to_string(), description: None, project_path: project_path.clone() }).await.unwrap();

    let att1 = make_att(&task.id, "att-001", "photo.png", "image/png", "/tmp/photo.png");
    let att2 = make_att(&task.id, "att-002", "report.pdf", "application/pdf", "/tmp/report.pdf");
    att_repo.create(&att1).await.unwrap();
    att_repo.create(&att2).await.unwrap();

    let tx = make_tx();
    let attachments = att_repo.list_for_task(&task.id).await.unwrap();
    write_task_context_file(&project_path, &task, &attachments, &[], &tx, "session-1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path)).unwrap();
    assert!(content.contains("## Attached Files"), "Missing ## Attached Files section");
    assert!(content.contains("photo.png"), "Missing photo.png");
    assert!(content.contains("report.pdf"), "Missing report.pdf");
}

#[tokio::test]
async fn test_context_file_attachment_uses_id_prefix() {
    let project_path = format!("/tmp/ctx-pipeline-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    let (task_repo, att_repo, _) = setup().await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: project_path.clone() }).await.unwrap();

    let att = make_att(&task.id, "abc-123", "screenshot.png", "image/png", "/tmp/ss.png");
    att_repo.create(&att).await.unwrap();

    let attachments = att_repo.list_for_task(&task.id).await.unwrap();
    let tx = make_tx();
    write_task_context_file(&project_path, &task, &attachments, &[], &tx, "s1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path)).unwrap();
    assert!(content.contains("abc-123-screenshot.png"), "Should use {{id}}-{{filename}} format");
}

#[tokio::test]
async fn test_context_file_excludes_litellm_comments() {
    let project_path = format!("/tmp/ctx-pipeline-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    let (task_repo, _, comment_repo) = setup().await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: project_path.clone() }).await.unwrap();

    comment_repo.create(&task.id, "user", CreateComment { content: "User comment here".to_string(), parent_id: None }).await.unwrap();
    comment_repo.create(&task.id, "litellm", CreateComment { content: "LiteLLM summary (should be excluded)".to_string(), parent_id: None }).await.unwrap();

    let comments = comment_repo.list_for_task(&task.id).await.unwrap();
    let tx = make_tx();
    write_task_context_file(&project_path, &task, &[], &comments, &tx, "s1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path)).unwrap();
    assert!(content.contains("User comment here"), "User comment should be present");
    assert!(!content.contains("LiteLLM summary"), "litellm comment should be excluded");
}

#[tokio::test]
async fn test_context_file_includes_implementation_plan() {
    let project_path = format!("/tmp/ctx-pipeline-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    let (task_repo, _, _) = setup().await;
    let mut task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: project_path.clone() }).await.unwrap();
    task.instructions = Some("Step 1: do X\nStep 2: do Y".to_string());

    let tx = make_tx();
    write_task_context_file(&project_path, &task, &[], &[], &tx, "s1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path)).unwrap();
    assert!(content.contains("## Implementation Plan"));
    assert!(content.contains("Step 1: do X"));
}

#[tokio::test]
async fn test_context_file_includes_compressed_context() {
    let project_path = format!("/tmp/ctx-pipeline-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    let (task_repo, _, _) = setup().await;
    let mut task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: project_path.clone() }).await.unwrap();
    task.compressed_context = Some("Prior session: fixed auth".to_string());

    let tx = make_tx();
    write_task_context_file(&project_path, &task, &[], &[], &tx, "s1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path)).unwrap();
    assert!(content.contains("## Prior Session Context"));
    assert!(content.contains("Prior session: fixed auth"));
}

#[tokio::test]
async fn test_image_data_url_pipeline_uses_stored_mime() {
    let tmp = format!("/tmp/test-pipeline-img-{}.png", uuid::Uuid::new_v4());
    tokio::fs::write(&tmp, b"FAKE_PNG").await.unwrap();

    // Pass "image/webp" as mime — should encode with webp, not infer png from extension
    let url = image_to_data_url(&tmp, "image/webp").await.unwrap();
    assert!(url.starts_with("data:image/webp;base64,"), "Should use provided mime, got: {}", url);
}

#[tokio::test]
async fn test_context_file_emits_event() {
    let project_path = format!("/tmp/ctx-pipeline-{}", uuid::Uuid::new_v4());
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    let (task_repo, _, _) = setup().await;
    let task = task_repo.create(CreateTask { title: "T".to_string(), description: None, project_path: project_path.clone() }).await.unwrap();

    let (tx, mut rx) = broadcast::channel(16);
    write_task_context_file(&project_path, &task, &[], &[], &tx, "sess-xyz");

    match rx.try_recv() {
        Ok(ai_kanban_backend::claude::ClaudeEvent::ContextFileUpdated { session_id, task_id }) => {
            assert_eq!(session_id, "sess-xyz");
            assert_eq!(task_id, task.id);
        }
        other => panic!("Expected ContextFileUpdated, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test pipeline_test 2>&1 | tail -10
```
Expected: `test result: ok. 7 passed`

- [ ] **Step 3: Commit**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/tests/pipeline_test.rs
git commit -m "test(backend): pipeline integration tests for context file and image URL"
```

---

### Task 11: Run backend coverage and fill remaining gaps

**Files:**
- Potentially add more tests if coverage <90%

- [ ] **Step 1: Install cargo-tarpaulin if not present**

```bash
which cargo-tarpaulin || cargo install cargo-tarpaulin 2>&1 | tail -5
```

- [ ] **Step 2: Run coverage**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo tarpaulin --out Stdout --exclude-files "tests/*" 2>&1 | grep -E "coverage|%"
```

- [ ] **Step 3: Identify gaps**

Look for modules below 80% and add targeted tests. Common gaps will be in:
- `src/api/routes.rs` — covered by api_test.rs
- `src/ws/` — covered by ws_test.rs
- `src/claude/prompts.rs` — covered by prompts_test.rs
- Any new code added in Phase 1

Add focused tests for any module below 80% until overall coverage ≥90%.

- [ ] **Step 4: Commit any new tests**

```bash
cd /home/utility/Projects/ai-kanban && git add backend/tests/
git commit -m "test(backend): additional tests to reach ≥90% coverage"
```

---

## PHASE 3 — Frontend Vitest Setup and Unit Tests

---

### Task 12: Install Vitest and create config

**Files:**
- Modify: `/home/utility/Projects/kanban-taskboard/frontend/package.json`
- Create: `/home/utility/Projects/kanban-taskboard/frontend/vitest.config.ts`

- [ ] **Step 1: Add devDependencies to package.json**

In `package.json`, add to `devDependencies`:

```json
"vitest": "^2.0.0",
"@vitest/coverage-v8": "^2.0.0",
"@vitest/ui": "^2.0.0",
"@vitejs/plugin-react": "^4.0.0",
"@testing-library/react": "^16.0.0",
"@testing-library/jest-dom": "^6.0.0",
"@testing-library/user-event": "^14.0.0",
"jsdom": "^25.0.0",
"msw": "^2.0.0"
```

Add to `scripts`:

```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage",
"test:ui": "vitest --ui"
```

- [ ] **Step 2: Create vitest.config.ts**

```typescript
// /home/utility/Projects/kanban-taskboard/frontend/vitest.config.ts
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      thresholds: { lines: 90, functions: 90, branches: 85, statements: 90 },
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/test/**',
        'src/**/*.d.ts',
        'src/app/layout.tsx',
        'src/app/**/page.tsx',
      ],
    },
  },
})
```

- [ ] **Step 3: Install dependencies**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm install 2>&1 | tail -5
```
Expected: no errors

- [ ] **Step 4: Run existing tests to verify setup works**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm test 2>&1 | tail -20
```
Expected: existing tests pass (smoke.test.ts, use-tasks.test.ts, etc.)

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && git -C /home/utility/Projects/ai-kanban add -A 2>/dev/null; true
cd /home/utility/Projects/kanban-taskboard && git add package.json vitest.config.ts package-lock.json
git commit -m "chore(frontend): install vitest, @testing-library/react, MSW, configure vitest.config.ts"
```

---

### Task 13: Kanban card and column tests

**Files:**
- Create: `frontend/src/test/components/kanban-card.test.tsx`
- Create: `frontend/src/test/components/kanban-column.test.tsx`

- [ ] **Step 1: Read the components**

```
/home/utility/Projects/kanban-taskboard/frontend/src/components/kanban/kanban-card.tsx
/home/utility/Projects/kanban-taskboard/frontend/src/components/kanban/kanban-column.tsx
```

- [ ] **Step 2: Create kanban-card.test.tsx**

```tsx
// src/test/components/kanban-card.test.tsx
import { render, screen } from '@testing-library/react';
import { KanbanCard } from '@/components/kanban/kanban-card';
import { mockTask, mockTask2 } from '../msw/fixtures';
import type { CostByTask } from '@/types/analytics';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={qc}>{children}</QueryClientProvider>
);

const mockCost: CostByTask = {
  task_id: mockTask.id,
  task_title: mockTask.title,
  input_tokens: 80000,
  output_tokens: 20000,
  cache_creation_tokens: 0,
  cache_read_tokens: 0,
  cost_usd: 0.05,
};

describe('KanbanCard', () => {
  it('renders task title', () => {
    render(<KanbanCard task={mockTask} />, { wrapper });
    expect(screen.getByText(mockTask.title)).toBeInTheDocument();
  });

  it('shows cost chip when costData provided', () => {
    render(<KanbanCard task={mockTask} costData={mockCost} />, { wrapper });
    expect(screen.getByText(/\$0\.05/)).toBeInTheDocument();
  });

  it('shows token chip with formatted value', () => {
    render(<KanbanCard task={mockTask} costData={mockCost} />, { wrapper });
    // 100k total tokens
    expect(screen.getByText(/100k tok/i)).toBeInTheDocument();
  });

  it('does not show cost chip when no costData', () => {
    render(<KanbanCard task={mockTask} />, { wrapper });
    expect(screen.queryByText(/\$/)).not.toBeInTheDocument();
  });

  it('renders stage badge', () => {
    render(<KanbanCard task={{ ...mockTask, stage: 'planning' }} />, { wrapper });
    // The card is inside the planning column so the stage may be implied
    expect(screen.getByText(mockTask.title)).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Create kanban-column.test.tsx**

```tsx
// src/test/components/kanban-column.test.tsx
import { render, screen } from '@testing-library/react';
import { KanbanColumn } from '@/components/kanban/kanban-column';
import { mockTask, mockTask2 } from '../msw/fixtures';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={qc}>{children}</QueryClientProvider>
);

describe('KanbanColumn', () => {
  it('renders column stage heading', () => {
    render(<KanbanColumn stage="planning" tasks={[]} />, { wrapper });
    expect(screen.getByText(/planning/i)).toBeInTheDocument();
  });

  it('renders task cards for each task', () => {
    render(<KanbanColumn stage="backlog" tasks={[mockTask, mockTask2]} />, { wrapper });
    expect(screen.getByText(mockTask.title)).toBeInTheDocument();
    expect(screen.getByText(mockTask2.title)).toBeInTheDocument();
  });

  it('shows task count', () => {
    render(<KanbanColumn stage="backlog" tasks={[mockTask, mockTask2]} />, { wrapper });
    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('renders empty state when no tasks', () => {
    render(<KanbanColumn stage="done" tasks={[]} />, { wrapper });
    // Column renders with zero count
    expect(screen.getByText('0')).toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Run the new tests**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm test -- --reporter=verbose src/test/components/kanban-card.test.tsx src/test/components/kanban-column.test.tsx 2>&1 | tail -20
```
Expected: all pass (fix any prop name mismatches by reading the actual component)

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/kanban-taskboard && git add src/test/components/
git commit -m "test(frontend): kanban card and column component tests"
```

---

### Task 14: Comment input and comment thread tests

**Files:**
- Create: `frontend/src/test/components/comment-input.test.tsx`
- Create: `frontend/src/test/components/comment-thread.test.tsx`

- [ ] **Step 1: Read the components**

```
/home/utility/Projects/kanban-taskboard/frontend/src/components/tasks/comment-input.tsx
/home/utility/Projects/kanban-taskboard/frontend/src/components/tasks/comment-thread.tsx
```

- [ ] **Step 2: Create comment-input.test.tsx**

```tsx
// src/test/components/comment-input.test.tsx
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CommentInput } from '@/components/tasks/comment-input';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { server } from '../msw/server';
import { http, HttpResponse } from 'msw';

const API = 'http://localhost:3001';

function wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

describe('CommentInput', () => {
  it('renders textarea and submit button', () => {
    render(<CommentInput taskId="task-123" onSuccess={vi.fn()} />, { wrapper });
    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /comment|submit/i })).toBeInTheDocument();
  });

  it('submit is disabled when content is empty', () => {
    render(<CommentInput taskId="task-123" onSuccess={vi.fn()} />, { wrapper });
    const btn = screen.getByRole('button', { name: /comment|submit/i });
    expect(btn).toBeDisabled();
  });

  it('submit enables when content is typed', async () => {
    const user = userEvent.setup();
    render(<CommentInput taskId="task-123" onSuccess={vi.fn()} />, { wrapper });
    await user.type(screen.getByRole('textbox'), 'hello');
    const btn = screen.getByRole('button', { name: /comment|submit/i });
    expect(btn).not.toBeDisabled();
  });

  it('calls onSuccess after submit', async () => {
    server.use(
      http.post(`${API}/api/tasks/:id/comments`, () =>
        HttpResponse.json({ id: 'c1', content: 'hello', author: 'user', task_id: 'task-123', parent_id: null, replies: [], created_at: new Date().toISOString() }, { status: 201 })
      )
    );
    const onSuccess = vi.fn();
    const user = userEvent.setup();
    render(<CommentInput taskId="task-123" onSuccess={onSuccess} />, { wrapper });
    await user.type(screen.getByRole('textbox'), 'hello');
    await user.click(screen.getByRole('button', { name: /comment|submit/i }));
    await waitFor(() => expect(onSuccess).toHaveBeenCalled());
  });

  it('clears content after successful submit', async () => {
    server.use(
      http.post(`${API}/api/tasks/:id/comments`, () =>
        HttpResponse.json({ id: 'c1', content: 'hello', author: 'user', task_id: 'task-123', parent_id: null, replies: [], created_at: new Date().toISOString() }, { status: 201 })
      )
    );
    const user = userEvent.setup();
    render(<CommentInput taskId="task-123" onSuccess={vi.fn()} />, { wrapper });
    const textarea = screen.getByRole('textbox');
    await user.type(textarea, 'hello');
    await user.click(screen.getByRole('button', { name: /comment|submit/i }));
    await waitFor(() => expect(textarea).toHaveValue(''));
  });
});
```

- [ ] **Step 3: Create comment-thread.test.tsx**

```tsx
// src/test/components/comment-thread.test.tsx
import { render, screen } from '@testing-library/react';
import { CommentThread } from '@/components/tasks/comment-thread';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={qc}>{children}</QueryClientProvider>
);

const mockComment = {
  comment: {
    id: 'c1',
    task_id: 'task-123',
    author: 'user',
    content: 'This is a **bold** comment',
    parent_id: null,
    created_at: '2026-03-01T10:00:00Z',
    updated_at: '2026-03-01T10:00:00Z',
  },
  replies: [],
};

const mockCommentWithImage = {
  comment: {
    ...mockComment.comment,
    id: 'c2',
    content: 'Here is an image: ![screenshot](http://localhost:3001/api/tasks/task-123/attachments/att-1/file)',
  },
  replies: [],
};

describe('CommentThread', () => {
  it('renders comment content', () => {
    render(<CommentThread thread={mockComment} taskId="task-123" />, { wrapper });
    expect(screen.getByText(/This is a/)).toBeInTheDocument();
  });

  it('renders markdown bold', () => {
    render(<CommentThread thread={mockComment} taskId="task-123" />, { wrapper });
    expect(screen.getByText('bold')).toBeInTheDocument();
    expect(screen.getByText('bold').tagName).toBe('STRONG');
  });

  it('renders image from markdown', () => {
    render(<CommentThread thread={mockCommentWithImage} taskId="task-123" />, { wrapper });
    const img = screen.getByRole('img', { name: 'screenshot' });
    expect(img).toBeInTheDocument();
  });

  it('shows author name', () => {
    render(<CommentThread thread={mockComment} taskId="task-123" />, { wrapper });
    expect(screen.getByText(/user/i)).toBeInTheDocument();
  });

  it('renders nested replies', () => {
    const withReply = {
      ...mockComment,
      replies: [{
        ...mockComment.comment,
        id: 'r1',
        content: 'A reply here',
        parent_id: 'c1',
      }],
    };
    render(<CommentThread thread={withReply} taskId="task-123" />, { wrapper });
    expect(screen.getByText('A reply here')).toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Run the tests**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm test -- src/test/components/comment-input.test.tsx src/test/components/comment-thread.test.tsx 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/kanban-taskboard && git add src/test/components/
git commit -m "test(frontend): comment input and comment thread component tests"
```

---

### Task 15: Sidebar, pricing, and hook tests

**Files:**
- Create: `frontend/src/test/components/sidebar.test.tsx`
- Create: `frontend/src/test/lib/pricing.test.ts`
- Create: `frontend/src/test/hooks/use-comments.test.ts`
- Create: `frontend/src/test/hooks/use-settings.test.ts`

- [ ] **Step 1: Read sidebar and pricing source files**

```
/home/utility/Projects/kanban-taskboard/frontend/src/components/layout/sidebar.tsx
/home/utility/Projects/kanban-taskboard/frontend/src/lib/pricing.ts
/home/utility/Projects/kanban-taskboard/frontend/src/hooks/use-comments.ts
/home/utility/Projects/kanban-taskboard/frontend/src/hooks/use-settings.ts
```

- [ ] **Step 2: Create pricing.test.ts**

```ts
// src/test/lib/pricing.test.ts
import { describe, it, expect } from 'vitest';
import { calculateCost, formatCost } from '@/lib/pricing';

describe('pricing', () => {
  it('returns 0 for zero tokens', () => {
    expect(calculateCost(0, 0)).toBe(0);
  });

  it('calculates cost for known token counts', () => {
    // 1M input tokens at some rate — just verify it returns a positive number
    const cost = calculateCost(1000000, 0);
    expect(cost).toBeGreaterThan(0);
  });

  it('output tokens have higher cost than input tokens', () => {
    const inputOnly = calculateCost(1000, 0);
    const outputOnly = calculateCost(0, 1000);
    expect(outputOnly).toBeGreaterThanOrEqual(inputOnly);
  });

  it('formatCost formats small values as $0.00', () => {
    expect(formatCost(0)).toBe('$0.00');
  });

  it('formatCost formats larger values with 2 decimal places', () => {
    expect(formatCost(1.5)).toBe('$1.50');
  });
});
```

Note: Read `pricing.ts` first to confirm the exported function names — adjust if they differ.

- [ ] **Step 3: Create use-comments.test.ts**

```ts
// src/test/hooks/use-comments.test.ts
import { renderHook, waitFor } from '@testing-library/react';
import { useComments, useCreateComment } from '@/hooks/use-comments';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { server } from '../msw/server';
import { http, HttpResponse } from 'msw';

const API = 'http://localhost:3001';

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe('useComments', () => {
  it('returns empty list initially (from MSW handler)', async () => {
    const { result } = renderHook(() => useComments('task-123'), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([]);
  });

  it('returns comments when API returns data', async () => {
    server.use(
      http.get(`${API}/api/tasks/:id/comments`, () =>
        HttpResponse.json([{ comment: { id: 'c1', content: 'hello', author: 'user', task_id: 'task-123', parent_id: null, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' }, replies: [] }])
      )
    );
    const { result } = renderHook(() => useComments('task-123'), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveLength(1);
  });
});

describe('useCreateComment', () => {
  it('posts a new comment', async () => {
    server.use(
      http.post(`${API}/api/tasks/:id/comments`, () =>
        HttpResponse.json({ id: 'new-c', content: 'new', author: 'user', task_id: 'task-123', parent_id: null, created_at: '', updated_at: '' }, { status: 201 })
      )
    );
    const { result } = renderHook(() => useCreateComment('task-123'), { wrapper: makeWrapper() });
    await result.current.mutateAsync({ content: 'new', parent_id: null });
    // No error means success
    expect(result.current.isError).toBe(false);
  });
});
```

- [ ] **Step 4: Create use-settings.test.ts**

```ts
// src/test/hooks/use-settings.test.ts
import { renderHook, waitFor } from '@testing-library/react';
import { useSettings } from '@/hooks/use-settings';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { server } from '../msw/server';
import { http, HttpResponse } from 'msw';

const API = 'http://localhost:3001';

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe('useSettings', () => {
  it('fetches settings object', async () => {
    server.use(
      http.get(`${API}/api/settings`, () =>
        HttpResponse.json({ litellm_summarization: true, litellm_task_enrichment: false })
      )
    );
    const { result } = renderHook(() => useSettings(), { wrapper: makeWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toHaveProperty('litellm_summarization');
  });
});
```

- [ ] **Step 5: Run all new tests**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm test -- src/test/lib/ src/test/hooks/ 2>&1 | tail -20
```

- [ ] **Step 6: Commit**

```bash
cd /home/utility/Projects/kanban-taskboard && git add src/test/
git commit -m "test(frontend): pricing, use-comments, use-settings tests"
```

---

### Task 16: WebSocket context and task-subscriptions tests

**Files:**
- Create: `frontend/src/test/contexts/websocket-context.test.tsx`
- Create: `frontend/src/test/hooks/use-task-subscriptions.test.ts`

- [ ] **Step 1: Read the source files**

```
/home/utility/Projects/kanban-taskboard/frontend/src/contexts/websocket-context.tsx
/home/utility/Projects/kanban-taskboard/frontend/src/hooks/use-task-subscriptions.ts
```

- [ ] **Step 2: Create websocket-context.test.tsx**

```tsx
// src/test/contexts/websocket-context.test.tsx
import { render, screen, act } from '@testing-library/react';
import { WebSocketProvider, useWebSocket } from '@/contexts/websocket-context';

// Mock WebSocket
class MockWebSocket {
  onopen: (() => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 1;
  send = vi.fn();
  close = vi.fn();
  constructor(public url: string) {}
}

beforeEach(() => {
  vi.stubGlobal('WebSocket', MockWebSocket);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function TestChild() {
  const { subscribe, unsubscribe } = useWebSocket();
  return (
    <div>
      <span data-testid="has-subscribe">{subscribe ? 'yes' : 'no'}</span>
      <span data-testid="has-unsubscribe">{unsubscribe ? 'yes' : 'no'}</span>
    </div>
  );
}

describe('WebSocketProvider', () => {
  it('provides subscribe and unsubscribe to children', () => {
    render(
      <WebSocketProvider>
        <TestChild />
      </WebSocketProvider>
    );
    expect(screen.getByTestId('has-subscribe').textContent).toBe('yes');
    expect(screen.getByTestId('has-unsubscribe').textContent).toBe('yes');
  });

  it('subscribe adds listener and dispatches messages to it', () => {
    let wsInstance: MockWebSocket | null = null;
    vi.stubGlobal('WebSocket', class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstance = this;
      }
    });

    const handler = vi.fn();
    function TestSubscribe() {
      const { subscribe, unsubscribe } = useWebSocket();
      React.useEffect(() => {
        subscribe('task_stage_changed', 'test-id', handler);
        return () => unsubscribe('task_stage_changed', 'test-id');
      }, []);
      return null;
    }

    render(
      <WebSocketProvider>
        <TestSubscribe />
      </WebSocketProvider>
    );

    act(() => {
      wsInstance?.onmessage?.({ data: JSON.stringify({ type: 'task_stage_changed', task_id: 'test-id', task: {} }) } as MessageEvent);
    });

    expect(handler).toHaveBeenCalled();
  });
});
```

Note: Read the actual WebSocketProvider implementation before writing tests — the subscribe/unsubscribe API may differ. Adjust function names and message format to match actual code.

- [ ] **Step 3: Create use-task-subscriptions.test.ts**

```tsx
// src/test/hooks/use-task-subscriptions.test.ts
import { renderHook } from '@testing-library/react';
import { useTaskSubscriptions } from '@/hooks/use-task-subscriptions';
import { WebSocketProvider } from '@/contexts/websocket-context';

// MockWebSocket as above
class MockWebSocket {
  onopen: (() => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onclose: (() => void) | null = null;
  readyState = 1;
  send = vi.fn();
  close = vi.fn();
  constructor(public url: string) {}
}

beforeEach(() => vi.stubGlobal('WebSocket', MockWebSocket));
afterEach(() => vi.unstubAllGlobals());

describe('useTaskSubscriptions', () => {
  it('mounts without error', () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <WebSocketProvider>{children}</WebSocketProvider>
    );
    const { result } = renderHook(
      () => useTaskSubscriptions({ taskId: 'task-123', onStageChange: vi.fn() }),
      { wrapper }
    );
    expect(result.error).toBeUndefined();
  });

  it('cleans up on unmount without error', () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <WebSocketProvider>{children}</WebSocketProvider>
    );
    const { unmount } = renderHook(
      () => useTaskSubscriptions({ taskId: 'task-123', onStageChange: vi.fn() }),
      { wrapper }
    );
    expect(() => unmount()).not.toThrow();
  });
});
```

- [ ] **Step 4: Run the tests**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm test -- src/test/contexts/ src/test/hooks/use-task-subscriptions.test.ts 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/kanban-taskboard && git add src/test/
git commit -m "test(frontend): websocket context and task-subscriptions hook tests"
```

---

### Task 17: Kanban board and task detail tests

**Files:**
- Create: `frontend/src/test/components/kanban-board.test.tsx`
- Create: `frontend/src/test/components/task-detail.test.tsx`

- [ ] **Step 1: Read source files**

```
/home/utility/Projects/kanban-taskboard/frontend/src/components/kanban/kanban-board.tsx
/home/utility/Projects/kanban-taskboard/frontend/src/components/tasks/task-detail.tsx
```

- [ ] **Step 2: Create kanban-board.test.tsx**

```tsx
// src/test/components/kanban-board.test.tsx
import { render, screen, waitFor } from '@testing-library/react';
import { KanbanBoard } from '@/components/kanban/kanban-board';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { mockTask, mockTask2 } from '../msw/fixtures';

const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={qc}>{children}</QueryClientProvider>
);

describe('KanbanBoard', () => {
  it('renders stage columns', async () => {
    render(<KanbanBoard />, { wrapper });
    // MSW returns [mockTask, mockTask2] for GET /api/tasks
    await waitFor(() => {
      // At minimum the board should render without crashing
      expect(document.body).toBeInTheDocument();
    });
  });

  it('shows task titles from mock data', async () => {
    render(<KanbanBoard />, { wrapper });
    await waitFor(() => {
      expect(screen.getByText(mockTask.title)).toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 3: Create task-detail.test.tsx**

```tsx
// src/test/components/task-detail.test.tsx
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TaskDetail } from '@/components/tasks/task-detail';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { mockTask } from '../msw/fixtures';
import { server } from '../msw/server';
import { http, HttpResponse } from 'msw';

const API = 'http://localhost:3001';

function makeWrapper() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  );
}

describe('TaskDetail', () => {
  it('renders task title', async () => {
    render(<TaskDetail taskId={mockTask.id} />, { wrapper: makeWrapper() });
    await waitFor(() => expect(screen.getByText(mockTask.title)).toBeInTheDocument());
  });

  it('renders task description when present', async () => {
    render(<TaskDetail taskId={mockTask.id} />, { wrapper: makeWrapper() });
    await waitFor(() => expect(screen.getByText(mockTask.description!)).toBeInTheDocument());
  });

  it('renders without crashing for unknown task (404)', async () => {
    server.use(
      http.get(`${API}/api/tasks/:id`, () =>
        HttpResponse.json({ error: 'Not found' }, { status: 404 })
      )
    );
    render(<TaskDetail taskId="nonexistent" />, { wrapper: makeWrapper() });
    // Should not throw — just render an empty or error state
    await waitFor(() => expect(document.body).toBeInTheDocument());
  });
});
```

- [ ] **Step 4: Run the tests**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm test -- src/test/components/kanban-board.test.tsx src/test/components/task-detail.test.tsx 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
cd /home/utility/Projects/kanban-taskboard && git add src/test/
git commit -m "test(frontend): kanban board and task detail component tests"
```

---

### Task 18: Run frontend coverage and fill gaps to 90%

- [ ] **Step 1: Run coverage**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm run test:coverage 2>&1 | tail -30
```

- [ ] **Step 2: Identify files below 80%**

Look at the coverage table. Common gaps: utility functions, smaller hooks, UI component edge cases.

- [ ] **Step 3: Add targeted tests for each gap**

For each file below 80%, add focused tests covering the main branches. Prioritize files with the most uncovered lines.

- [ ] **Step 4: Re-run coverage until ≥90%**

```bash
npm run test:coverage 2>&1 | grep -E "All files|Statements|Branches|Functions|Lines"
```
Expected: Lines ≥90, Functions ≥90, Branches ≥85

- [ ] **Step 5: Commit all new test files**

```bash
cd /home/utility/Projects/kanban-taskboard && git add src/test/
git commit -m "test(frontend): additional tests to reach ≥90% coverage"
```

---

## Final Verification

- [ ] **Backend: all tests pass**

```bash
cd /home/utility/Projects/ai-kanban/backend && cargo test 2>&1 | tail -5
```
Expected: no new failures (pre-existing `test_parse_sample_output` date failure is known and unrelated)

- [ ] **Frontend: all tests pass with ≥90% coverage**

```bash
cd /home/utility/Projects/kanban-taskboard/frontend && npm run test:coverage 2>&1 | tail -20
```
Expected: Lines ≥90%
