use super::TaskApiState;
use crate::models::{CommentWithReplies, CreateTask, UpdateTask};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::collections::HashSet;
use tracing::{debug, error, info, instrument, warn};

#[derive(Deserialize, Debug)]
struct ListQuery {
    stage: Option<String>,
}

#[derive(Deserialize, Debug)]
struct MoveRequest {
    stage: String,
}

pub fn task_routes() -> Router<TaskApiState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/:id", get(get_task).patch(update_task).delete(delete_task))
        .route("/:id/move", post(move_task))
        .route("/:id/sessions", post(start_session))
        .route("/:id/sessions/continue", post(continue_session))
        .route("/:id/sessions-detail", get(task_sessions_detail))
        .route("/:id/context-file", get(get_context_file))
}

#[instrument(skip(state))]
async fn list_tasks(
    State(state): State<TaskApiState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    debug!(stage_filter = ?query.stage, "Listing tasks");
    match state.repo.list(query.stage.as_deref()).await {
        Ok(tasks) => {
            debug!(count = tasks.len(), stage_filter = ?query.stage, "Tasks retrieved");
            Json(tasks).into_response()
        }
        Err(e) => {
            error!(stage_filter = ?query.stage, error = %e, "Failed to list tasks");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn get_task(State(state): State<TaskApiState>, Path(id): Path<String>) -> impl IntoResponse {
    debug!(task_id = %id, "Getting task");
    match state.repo.find(&id).await {
        Ok(task) => {
            debug!(task_id = %id, title = %task.title, stage = %task.stage, "Task retrieved");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Task not found");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn create_task(
    State(state): State<TaskApiState>,
    Json(create): Json<CreateTask>,
) -> impl IntoResponse {
    info!(title = %create.title, project_path = %create.project_path, "Creating task");

    // Auto-create ~/Projects/<name> if the path starts with ~/Projects/
    if let Some(name) = create.project_path.strip_prefix("~/Projects/") {
        // Security: reject any path component that could escape the Projects directory
        if name.contains('/') || name.contains('\\') || name.contains("..") || name.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid project path: use a simple directory name under ~/Projects/" })),
            ).into_response();
        }
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let full_path = std::path::PathBuf::from(home).join("Projects").join(name);
            if !full_path.exists() {
                if let Err(e) = std::fs::create_dir_all(&full_path) {
                    warn!(path = %full_path.display(), error = %e, "Failed to create project directory");
                } else {
                    info!(path = %full_path.display(), "Created project directory");
                }
            }
        }
    }

    match state.repo.create(create).await {
        Ok(task) => {
            info!(task_id = %task.id, title = %task.title, stage = %task.stage, project_path = %task.project_path, "Task created");
            (StatusCode::CREATED, Json(task)).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to create task");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn update_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
    Json(update): Json<UpdateTask>,
) -> impl IntoResponse {
    info!(
        task_id = %id,
        update_title = ?update.title,
        update_stage = ?update.stage,
        update_context = update.context.is_some(),
        update_description = update.description.is_some(),
        "Updating task"
    );
    match state.repo.update(&id, update).await {
        Ok(task) => {
            info!(task_id = %id, title = %task.title, stage = %task.stage, "Task updated");
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to update task");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn delete_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "Deleting task");
    // Remove from pending queue and stop any running Claude process for this task
    if let Some(queue) = &state.queue {
        let dequeued = queue.dequeue(&id).await;
        if dequeued {
            info!(task_id = %id, "Removed task from pending queue before delete");
        }
        if let Some(session_id) = queue.get_active_session_for_task(&id).await {
            info!(task_id = %id, session_id = %session_id, "Stopping active session before task delete");
            let _ = queue.stop_session(&session_id).await;
        }
    }
    match state.repo.delete(&id).await {
        Ok(()) => {
            info!(task_id = %id, "Task deleted");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to delete task");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn start_session(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "Starting Claude session for task");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => {
            error!(task_id = %id, error = %e, "Task not found for session start");
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => {
            error!(task_id = %id, "Session queue not initialized");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Session queue not available" })),
            )
                .into_response();
        }
    };

    let stage = task.stage.clone();
    info!(task_id = %id, title = %task.title, stage = %stage, "Enqueuing task for Claude session");
    match queue.enqueue(task, stage, None, None, 0).await {
        Ok(()) => {
            info!(task_id = %id, "Task enqueued for Claude session");
            Json(serde_json::json!({ "status": "queued" })).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to enqueue task for Claude session");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// Build the comment context string for injecting into a Claude session.
///
/// cutoff=None: include all non-litellm comments and their replies.
/// cutoff=Some(dt):
///   - Rule 3 (first): include comments where parent.created_at > dt (with all their replies).
///   - Rule 2 (second): for any remaining parent with replies where reply.created_at > dt,
///     include the parent as `[context]` plus only the qualifying replies.
pub fn build_comment_history(
    comments: &[CommentWithReplies],
    cutoff: Option<chrono::DateTime<chrono::Utc>>,
) -> Option<String> {
    fn fmt(author: &str, content: &str) -> String {
        let prefix = if author == "claude" {
            "[Claude]"
        } else {
            "[You]"
        };
        format!("{}: {}", prefix, content)
    }

    let lines: Vec<String> = match cutoff {
        None => comments
            .iter()
            .filter(|c| c.comment.author != "litellm")
            .flat_map(|c| {
                let mut ls = vec![fmt(&c.comment.author, &c.comment.content)];
                for r in c.replies.iter().filter(|r| r.author != "litellm") {
                    ls.push(format!("  {}", fmt(&r.author, &r.content)));
                }
                ls
            })
            .collect(),

        Some(cutoff_dt) => {
            let mut included_ids: HashSet<String> = HashSet::new();
            let mut ls: Vec<String> = Vec::new();

            // Rule 3: new top-level comments (parent.created_at > cutoff)
            for c in comments
                .iter()
                .filter(|c| c.comment.author != "litellm" && c.comment.created_at > cutoff_dt)
            {
                included_ids.insert(c.comment.id.clone());
                ls.push(fmt(&c.comment.author, &c.comment.content));
                for r in c.replies.iter().filter(|r| r.author != "litellm") {
                    ls.push(format!("  {}", fmt(&r.author, &r.content)));
                }
            }

            // Rule 2: old parents that have new replies
            for c in comments
                .iter()
                .filter(|c| c.comment.author != "litellm" && !included_ids.contains(&c.comment.id))
            {
                let new_replies: Vec<_> = c
                    .replies
                    .iter()
                    .filter(|r| r.author != "litellm" && r.created_at > cutoff_dt)
                    .collect();
                if !new_replies.is_empty() {
                    ls.push(format!(
                        "[context] {}",
                        fmt(&c.comment.author, &c.comment.content)
                    ));
                    for r in &new_replies {
                        ls.push(format!("  {}", fmt(&r.author, &r.content)));
                    }
                }
            }

            ls
        }
    };

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

#[instrument(skip(state))]
async fn continue_session(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "Continuing Claude session with comment history");

    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => {
            error!(task_id = %id, error = %e, "Task not found for continue session");
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let queue = match &state.queue {
        Some(q) => q.clone(),
        None => {
            error!(task_id = %id, "Session queue not available for continue session");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Queue not available" })),
            )
                .into_response();
        }
    };

    // Look up prior session for true resume.
    // When --resume is used, Claude restores its full internal conversation state,
    // so we only need to inject comments added AFTER that session ended (not the whole history).
    let (resume_claude_session_id, prior_session_ended_at) = if let Some(ref prior_session_id) =
        task.session_id
    {
        match state.session_repo.find(prior_session_id).await {
            Ok(prior_session) => {
                if let Some(ref csid) = prior_session.claude_session_id {
                    info!(task_id = %id, claude_session_id = %csid, "Found prior claude_session_id for resume");
                } else {
                    info!(task_id = %id, "No claude_session_id on prior session — will inject full context");
                }
                let ended_at = prior_session.ended_at;
                (prior_session.claude_session_id, ended_at)
            }
            Err(e) => {
                tracing::warn!(task_id = %id, error = %e, "Could not fetch prior session; falling back to context injection");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // Build conversation context from comment thread.
    // Filter out litellm-authored summaries — Claude doesn't need to read its own summaries.
    // When doing a true --resume, only include comments added AFTER the prior session ended,
    // because Claude's internal state (via --resume) already has everything up to that point.
    let comments = state
        .comment_repo
        .list_for_task(&id)
        .await
        .unwrap_or_default();
    let total_comments = comments.len();
    let total_replies: usize = comments.iter().map(|c| c.replies.len()).sum();

    let cutoff = if resume_claude_session_id.is_some() {
        prior_session_ended_at
    } else {
        None
    };

    let comment_history = build_comment_history(&comments, cutoff);
    let new_comment_count = comment_history
        .as_ref()
        .map(|h| h.lines().count())
        .unwrap_or(0);

    // Prepend compressed context if available (from prior high-token sessions).
    // Only include compressed context when NOT doing a true --resume (avoid duplication).
    let conversation_context = if resume_claude_session_id.is_some() {
        // True resume: only inject new comments, Claude already has full prior context
        comment_history.map(|h| format!("## New messages since last session:\n{h}"))
    } else {
        // Context injection (no --resume): include full compressed context + history
        match (&task.compressed_context, &comment_history) {
            (Some(compressed), Some(history)) => Some(format!(
                "## Prior session context (compressed):\n{compressed}\n\n## Recent conversation:\n{history}"
            )),
            (Some(compressed), None) => Some(format!(
                "## Prior session context (compressed):\n{compressed}"
            )),
            (None, Some(history)) => Some(history.clone()),
            (None, None) => None,
        }
    };

    let stage = task.stage.clone();
    info!(
        task_id = %id,
        title = %task.title,
        stage = %stage,
        total_comments = total_comments,
        reply_count = total_replies,
        new_comments_injected = new_comment_count,
        has_context = conversation_context.is_some(),
        has_resume = resume_claude_session_id.is_some(),
        "Enqueuing continue session"
    );
    match queue
        .enqueue(task, stage, conversation_context, resume_claude_session_id, 0)
        .await
    {
        Ok(()) => {
            info!(task_id = %id, "Continue session enqueued");
            Json(serde_json::json!({ "status": "queued" })).into_response()
        }
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to enqueue continue session");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn task_sessions_detail(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!(task_id = %id, "API: Getting sessions detail for task");
    match state.session_repo.list_by_task_with_tokens(&id).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to get sessions detail");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn move_task(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
    Json(body): Json<MoveRequest>,
) -> impl IntoResponse {
    info!(task_id = %id, to_stage = %body.stage, "Moving task to new stage");
    match state.repo.move_to_stage(&id, &body.stage).await {
        Ok(task) => {
            info!(task_id = %id, title = %task.title, stage = %task.stage, "Task moved to stage");
            // Auto-stop session when task moves to Done
            if body.stage == "done" {
                if let Some(queue) = &state.queue {
                    if let Some(session_id) = queue.get_active_session_for_task(&id).await {
                        info!(task_id = %id, session_id = %session_id, "Auto-stopping session — task moved to Done");
                        let _ = queue.stop_session(&session_id).await;
                    }
                }
            }
            Json(task).into_response()
        }
        Err(e) => {
            error!(task_id = %id, to_stage = %body.stage, error = %e, "Failed to move task to stage");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[instrument(skip(state))]
async fn get_context_file(
    State(state): State<TaskApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let task = match state.repo.find(&id).await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let expanded = if task.project_path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{}/{}", home, &task.project_path[2..])
    } else {
        task.project_path.clone()
    };
    let file_path = std::path::Path::new(&expanded)
        .join(".claude")
        .join("ai-kanban.md");
    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            Json(serde_json::json!({ "content": content, "path": file_path.to_string_lossy() }))
                .into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::json!({ "error": "Context file not found — session not yet started" }),
            ),
        )
            .into_response(),
        Err(e) => {
            error!(task_id = %id, error = %e, "Failed to read context file");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_comment_history;
    use crate::models::{Comment, CommentWithReplies};
    use chrono::{DateTime, TimeZone, Utc};

    fn make_comment(id: &str, author: &str, content: &str, ts: DateTime<Utc>) -> Comment {
        Comment {
            id: id.to_string(),
            task_id: "t1".to_string(),
            parent_id: None,
            author: author.to_string(),
            content: content.to_string(),
            created_at: ts,
        }
    }

    fn make_reply(
        id: &str,
        parent_id: &str,
        author: &str,
        content: &str,
        ts: DateTime<Utc>,
    ) -> Comment {
        Comment {
            id: id.to_string(),
            task_id: "t1".to_string(),
            parent_id: Some(parent_id.to_string()),
            author: author.to_string(),
            content: content.to_string(),
            created_at: ts,
        }
    }

    #[test]
    fn test_no_cutoff_includes_all_comments_and_replies() {
        let t = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let comments = vec![CommentWithReplies {
            comment: make_comment("c1", "user", "hello", t),
            replies: vec![make_reply("r1", "c1", "claude", "hi", t)],
        }];
        let result = build_comment_history(&comments, None).unwrap();
        assert!(result.contains("[You]: hello"));
        assert!(result.contains("[Claude]: hi"));
    }

    #[test]
    fn test_cutoff_excludes_old_parent_with_no_new_replies() {
        let old = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();
        let comments = vec![CommentWithReplies {
            comment: make_comment("c1", "user", "old comment", old),
            replies: vec![],
        }];
        let result = build_comment_history(&comments, Some(cutoff));
        assert!(result.is_none());
    }

    #[test]
    fn test_cutoff_includes_old_parent_as_context_when_new_reply_exists() {
        let old = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let new_ts = Utc.with_ymd_and_hms(2026, 3, 3, 10, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();
        let comments = vec![CommentWithReplies {
            comment: make_comment("c1", "user", "old parent", old),
            replies: vec![make_reply("r1", "c1", "user", "new reply", new_ts)],
        }];
        let result = build_comment_history(&comments, Some(cutoff)).unwrap();
        assert!(
            result.contains("[context]"),
            "Should include parent as context"
        );
        assert!(result.contains("old parent"));
        assert!(result.contains("new reply"));
    }

    #[test]
    fn test_cutoff_excludes_old_reply_on_old_parent() {
        let old = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();
        let comments = vec![CommentWithReplies {
            comment: make_comment("c1", "user", "old parent", old),
            replies: vec![make_reply("r1", "c1", "user", "old reply", old)],
        }];
        let result = build_comment_history(&comments, Some(cutoff));
        assert!(result.is_none());
    }

    #[test]
    fn test_litellm_comments_excluded() {
        let t = Utc.with_ymd_and_hms(2026, 3, 3, 10, 0, 0).unwrap();
        let comments = vec![CommentWithReplies {
            comment: make_comment("c1", "litellm", "summary", t),
            replies: vec![],
        }];
        let result = build_comment_history(&comments, None);
        assert!(result.is_none());
    }
}
