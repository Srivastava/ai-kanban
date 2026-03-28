use ai_kanban_backend::ai::context_manager::ContextManager;
use ai_kanban_backend::ai::litellm::LitellmClient;
use ai_kanban_backend::db::{create_pool, AttachmentRepository, CommentRepository, TaskRepository};
use ai_kanban_backend::models::CreateTask;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

async fn setup_db() -> (
    sqlx::SqlitePool,
    TaskRepository,
    CommentRepository,
    AttachmentRepository,
) {
    let db_path = format!("/tmp/test-ctx-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("create pool");
    let task_repo = TaskRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());
    (pool, task_repo, comment_repo, attachment_repo)
}

fn make_ctx(
    mock_uri: &str,
    comment_repo: CommentRepository,
    task_repo: TaskRepository,
    attachment_repo: AttachmentRepository,
) -> ContextManager {
    let litellm = LitellmClient::new(mock_uri, "test-key", "test-model");
    ContextManager::new(litellm, comment_repo, task_repo, attachment_repo)
}

fn ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "choices": [{"message": {"content": "summary text"}}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5}
    }))
}

// ---------------------------------------------------------------------------
// summarize_session — empty display_lines + empty result_text skips LLM
// ---------------------------------------------------------------------------

#[tokio::test]
async fn summarize_session_empty_lines_skips_llm() {
    let mock_server = MockServer::start().await;
    // Mount a mock that would fail the test if called
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("should not be called"))
        .expect(0)
        .mount(&mock_server)
        .await;

    let (_, task_repo, comment_repo, attachment_repo) = setup_db().await;
    let ctx = make_ctx(&mock_server.uri(), comment_repo, task_repo, attachment_repo);

    let result = ctx
        .summarize_session(
            "sess-1",
            "task-1",
            "My Task",
            "planning",
            Some(30),
            0,
            0,
            &[],  // empty lines
            None, // no result_text
        )
        .await;

    assert!(
        result.is_ok(),
        "expected Ok when lines are empty, got {:?}",
        result.err()
    );
    // wiremock will verify 0 requests on drop
}

// ---------------------------------------------------------------------------
// summarize_session — non-empty lines → LiteLLM called → comment posted
// ---------------------------------------------------------------------------

#[tokio::test]
async fn summarize_session_posts_comment() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ok_response())
        .mount(&mock_server)
        .await;

    let (pool, task_repo, comment_repo, attachment_repo) = setup_db().await;

    // Create a real task so the comment FK is satisfied
    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    let ctx = make_ctx(
        &mock_server.uri(),
        comment_repo.clone(),
        task_repo,
        attachment_repo,
    );

    let lines = vec!["did some work".to_string(), "edited a file".to_string()];
    let result = ctx
        .summarize_session(
            "sess-2",
            &task.id,
            &task.title,
            "planning",
            Some(60),
            100,
            50,
            &lines,
            Some("final output"),
        )
        .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());

    // Verify a comment was created for this task
    let comments = comment_repo
        .list_for_task(&task.id)
        .await
        .expect("list comments");
    assert_eq!(comments.len(), 1, "expected exactly 1 comment posted");
    assert!(
        comments[0].comment.content.contains("Session Summary"),
        "comment should contain 'Session Summary'"
    );
    assert!(
        comments[0].comment.content.contains("summary text"),
        "comment should contain the LLM response"
    );
    assert_eq!(
        comments[0].comment.author, "litellm",
        "comment author should be 'litellm'"
    );

    let _ = pool;
}

// ---------------------------------------------------------------------------
// enrich_task — LiteLLM 500 → returns Err
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enrich_task_llm_error_returns_err() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let (_, task_repo, comment_repo, attachment_repo) = setup_db().await;
    let ctx = make_ctx(&mock_server.uri(), comment_repo, task_repo, attachment_repo);

    let result = ctx
        .enrich_task("task-err", "Some Task", Some("description here"))
        .await;

    assert!(result.is_err(), "expected Err on LiteLLM 500, got Ok");
}

// ---------------------------------------------------------------------------
// enrich_task — valid LLM response → task instructions updated in DB
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enrich_task_valid_response_updates_task() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "enriched instructions here"}}],
            "usage": {"prompt_tokens": 20, "completion_tokens": 8}
        })))
        .mount(&mock_server)
        .await;

    let (_, task_repo, comment_repo, attachment_repo) = setup_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Enrich Me".to_string(),
            description: Some("brief desc".to_string()),
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    let ctx = make_ctx(
        &mock_server.uri(),
        comment_repo,
        task_repo.clone(),
        attachment_repo,
    );

    let result = ctx
        .enrich_task(&task.id, &task.title, task.description.as_deref())
        .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    let enriched_opt = result.unwrap();
    assert!(enriched_opt.is_some(), "expected Some(enriched text)");
    let enriched = enriched_opt.unwrap();
    assert!(
        enriched.contains("enriched instructions here"),
        "returned text should contain LLM content"
    );

    // Verify persisted in DB
    let updated = task_repo.find(&task.id).await.expect("find task");
    assert_eq!(
        updated.instructions.as_deref(),
        Some("enriched instructions here"),
        "task instructions should be updated in DB"
    );
}

// ---------------------------------------------------------------------------
// compress_context — valid LLM response → compressed_context stored on task
// ---------------------------------------------------------------------------

#[tokio::test]
async fn compress_context_stores_result() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "compressed context content"}}],
            "usage": {"prompt_tokens": 30, "completion_tokens": 12}
        })))
        .mount(&mock_server)
        .await;

    let (_, task_repo, comment_repo, attachment_repo) = setup_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Compress Me".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    let ctx = make_ctx(
        &mock_server.uri(),
        comment_repo,
        task_repo.clone(),
        attachment_repo,
    );

    let lines = vec!["activity line 1".to_string(), "activity line 2".to_string()];
    let result = ctx
        .compress_context("sess-3", &task.id, &task.title, &lines, Some("result text"))
        .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());

    let updated = task_repo.find(&task.id).await.expect("find task");
    let compressed = updated
        .compressed_context
        .expect("compressed_context should be set");
    assert!(
        compressed.contains("compressed context content"),
        "compressed_context should contain LLM response"
    );
}

// ---------------------------------------------------------------------------
// generate_briefing — valid LLM response → returns the briefing string
// ---------------------------------------------------------------------------

#[tokio::test]
async fn generate_briefing_returns_content() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "briefing content here"}}],
            "usage": {"prompt_tokens": 15, "completion_tokens": 6}
        })))
        .mount(&mock_server)
        .await;

    let (_, task_repo, comment_repo, attachment_repo) = setup_db().await;
    let ctx = make_ctx(&mock_server.uri(), comment_repo, task_repo, attachment_repo);

    let result = ctx
        .generate_briefing("My Task Title", "long conversation history...")
        .await;

    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    let briefing = result.unwrap();
    assert!(
        briefing.contains("briefing content here"),
        "briefing should contain LLM response content"
    );
    assert!(
        briefing.contains("Briefing compressed by LiteLLM"),
        "briefing should contain the LiteLLM prefix"
    );
}

// ---------------------------------------------------------------------------
// generate_briefing — LiteLLM 500 → returns Err
// ---------------------------------------------------------------------------

#[tokio::test]
async fn generate_briefing_llm_error_returns_err() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("error"))
        .mount(&mock_server)
        .await;

    let (_, task_repo, comment_repo, attachment_repo) = setup_db().await;
    let ctx = make_ctx(&mock_server.uri(), comment_repo, task_repo, attachment_repo);

    let result = ctx.generate_briefing("Task", "context").await;
    assert!(result.is_err(), "expected Err on LiteLLM 500");
}
