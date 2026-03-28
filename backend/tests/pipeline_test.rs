/// End-to-end pipeline integration tests for the attachment/context pipeline.
///
/// Tests exercise the interaction between attachments, tasks, context files,
/// and data URLs using a real SQLite DB and temp directories.
use ai_kanban_backend::claude::{write_task_context_file, ClaudeEvent};
use ai_kanban_backend::db::{create_pool, AttachmentRepository, CommentRepository, TaskRepository};
use ai_kanban_backend::models::{CreateComment, CreateTask, TaskAttachment};
use chrono::Utc;
use tokio::sync::broadcast;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

async fn setup_db() -> (TaskRepository, CommentRepository, AttachmentRepository) {
    let db_path = format!("/tmp/test-pipeline-{}.db", Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("create pool");
    let task_repo = TaskRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());
    (task_repo, comment_repo, attachment_repo)
}

fn make_broadcast() -> broadcast::Sender<ClaudeEvent> {
    let (tx, _) = broadcast::channel(16);
    tx
}

fn unique_tmp_dir(prefix: &str) -> String {
    format!("/tmp/{}-{}", prefix, Uuid::new_v4())
}

fn make_attachment(
    task_id: &str,
    filename: &str,
    mime_type: &str,
    storage_path: &str,
) -> TaskAttachment {
    TaskAttachment {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        filename: filename.to_string(),
        storage_path: storage_path.to_string(),
        mime_type: mime_type.to_string(),
        size_bytes: 1024,
        created_at: Utc::now(),
    }
}

fn read_context_file(project_path: &str) -> String {
    std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path))
        .expect("context file should exist")
}

// ---------------------------------------------------------------------------
// Test 1: context file has Attached Files section with both attachments listed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn context_file_has_attached_files_section() {
    let (task_repo, _comment_repo, attachment_repo) = setup_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Attached Files Task".to_string(),
            description: Some("A task with attachments".to_string()),
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    let img_att = make_attachment(
        &task.id,
        "screenshot.png",
        "image/png",
        "/uploads/screenshot.png",
    );
    let pdf_att = make_attachment(&task.id, "spec.pdf", "application/pdf", "/uploads/spec.pdf");

    attachment_repo
        .create(&img_att)
        .await
        .expect("create image attachment");
    attachment_repo
        .create(&pdf_att)
        .await
        .expect("create pdf attachment");

    let attachments = attachment_repo
        .list_for_task(&task.id)
        .await
        .expect("list attachments");
    assert_eq!(attachments.len(), 2);

    let project_path = unique_tmp_dir("pipeline-test1");
    let tx = make_broadcast();

    write_task_context_file(&project_path, &task, &attachments, &[], &tx, "session-1");

    let content = read_context_file(&project_path);

    assert!(
        content.contains("## Attached Files"),
        "context file should have '## Attached Files' section, got:\n{}",
        content
    );
    assert!(
        content.contains("screenshot.png"),
        "context file should mention screenshot.png"
    );
    assert!(
        content.contains("spec.pdf"),
        "context file should mention spec.pdf"
    );
    assert!(
        content.contains("image/png"),
        "context file should include image/png mime type"
    );
    assert!(
        content.contains("application/pdf"),
        "context file should include application/pdf mime type"
    );
}

// ---------------------------------------------------------------------------
// Test 2: image attachment uses {id}-{filename} format in context file
// ---------------------------------------------------------------------------

#[tokio::test]
async fn image_attachment_uses_id_filename_in_context() {
    let (task_repo, _comment_repo, attachment_repo) = setup_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "ID Filename Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    let att = make_attachment(&task.id, "photo.png", "image/png", "/uploads/photo.png");
    let att_id = att.id.clone();

    attachment_repo
        .create(&att)
        .await
        .expect("create attachment");

    let attachments = attachment_repo
        .list_for_task(&task.id)
        .await
        .expect("list attachments");

    let project_path = unique_tmp_dir("pipeline-test2");
    let tx = make_broadcast();

    write_task_context_file(&project_path, &task, &attachments, &[], &tx, "session-2");

    let content = read_context_file(&project_path);

    // Should reference {id}-{filename} NOT just {filename}
    let expected_ref = format!(".claude/attachments/{}-photo.png", att_id);
    assert!(
        content.contains(&expected_ref),
        "context file should reference '{}' but got:\n{}",
        expected_ref,
        content
    );

    // Should NOT contain bare filename without ID prefix
    assert!(
        !content.contains(".claude/attachments/photo.png")
            || content.contains(&format!("{}-photo.png", att_id)),
        "context file should use id-prefixed filename"
    );
}

// ---------------------------------------------------------------------------
// Test 3: litellm comments excluded from Discussion section
// ---------------------------------------------------------------------------

#[tokio::test]
async fn litellm_comment_excluded_from_discussion() {
    let (task_repo, comment_repo, _attachment_repo) = setup_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Discussion Filter Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    // Create 2 user comments
    comment_repo
        .create(
            &task.id,
            "user",
            CreateComment {
                content: "This is a user comment alpha".to_string(),
                parent_id: None,
            },
        )
        .await
        .expect("create user comment 1");

    comment_repo
        .create(
            &task.id,
            "user",
            CreateComment {
                content: "This is a user comment beta".to_string(),
                parent_id: None,
            },
        )
        .await
        .expect("create user comment 2");

    // Create 1 litellm comment — should be excluded
    comment_repo
        .create(
            &task.id,
            "litellm",
            CreateComment {
                content: "This is a litellm summary comment".to_string(),
                parent_id: None,
            },
        )
        .await
        .expect("create litellm comment");

    let comments = comment_repo
        .list_for_task(&task.id)
        .await
        .expect("list comments");
    assert_eq!(comments.len(), 3, "should have 3 comments total");

    let project_path = unique_tmp_dir("pipeline-test3");
    let tx = make_broadcast();

    write_task_context_file(&project_path, &task, &[], &comments, &tx, "session-3");

    let content = read_context_file(&project_path);

    assert!(
        content.contains("## Discussion"),
        "context file should have Discussion section"
    );
    assert!(
        content.contains("user comment alpha"),
        "user comment alpha should be in Discussion"
    );
    assert!(
        content.contains("user comment beta"),
        "user comment beta should be in Discussion"
    );
    assert!(
        !content.contains("litellm summary comment"),
        "litellm comment should NOT be in Discussion, but got:\n{}",
        content
    );
}

// ---------------------------------------------------------------------------
// Test 4: Implementation Plan section present when instructions set
// ---------------------------------------------------------------------------

#[tokio::test]
async fn instructions_section_present_when_set() {
    let (task_repo, _comment_repo, _attachment_repo) = setup_db().await;

    let mut task = task_repo
        .create(CreateTask {
            title: "Instructions Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    // Set instructions on the task directly (simulating an update)
    task.instructions = Some("Step 1: Do the thing\nStep 2: Verify it works".to_string());

    let project_path = unique_tmp_dir("pipeline-test4");
    let tx = make_broadcast();

    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-4");

    let content = read_context_file(&project_path);

    assert!(
        content.contains("## Implementation Plan"),
        "context file should have '## Implementation Plan' section, got:\n{}",
        content
    );
    assert!(
        content.contains("Step 1: Do the thing"),
        "context file should contain the instructions content"
    );
    assert!(
        content.contains("Step 2: Verify it works"),
        "context file should contain both instruction steps"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Prior Session Context section present when compressed_context set
// ---------------------------------------------------------------------------

#[tokio::test]
async fn compressed_context_section_present() {
    let (task_repo, _comment_repo, _attachment_repo) = setup_db().await;

    let mut task = task_repo
        .create(CreateTask {
            title: "Compressed Context Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    task.compressed_context =
        Some("Previously, Claude refactored the DB layer and added tests.".to_string());

    let project_path = unique_tmp_dir("pipeline-test5");
    let tx = make_broadcast();

    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-5");

    let content = read_context_file(&project_path);

    assert!(
        content.contains("## Prior Session Context"),
        "context file should have '## Prior Session Context' section, got:\n{}",
        content
    );
    assert!(
        content.contains("Claude refactored the DB layer"),
        "context file should contain the compressed context content"
    );
}

// ---------------------------------------------------------------------------
// Test 6: task_image_data_urls returns base64 data URL for real image file
// ---------------------------------------------------------------------------

#[tokio::test]
async fn image_to_data_url_returns_base64() {
    // Create a real PNG file on disk (minimal valid PNG bytes)
    let tmp_img_path = format!("/tmp/test-pipeline-img-{}.png", Uuid::new_v4());

    // Minimal 1x1 red PNG (89 bytes)
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // width=1, height=1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth=8, color=2(RGB), CRC
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, // IDAT data
        0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, // IDAT data + CRC
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
        0x44, 0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ];

    std::fs::write(&tmp_img_path, png_bytes).expect("write test PNG");

    // Call image_to_data_url directly (it's the same function task_image_data_urls uses)
    let result =
        ai_kanban_backend::ai::litellm::image_to_data_url(&tmp_img_path, "image/png").await;

    assert!(
        result.is_some(),
        "image_to_data_url should return Some for a valid image file"
    );
    let data_url = result.unwrap();

    assert!(
        data_url.starts_with("data:image/png;base64,"),
        "data URL should start with 'data:image/png;base64,', got: {}",
        &data_url[..data_url.len().min(60)]
    );
    assert!(
        data_url.len() > 30,
        "data URL should contain actual base64 data"
    );

    // Cleanup
    let _ = std::fs::remove_file(&tmp_img_path);
}

// ---------------------------------------------------------------------------
// Test 6b: task_image_data_urls via ContextManager integration
//          Uses a real attachment with real file on disk and verifies
//          the data URL is passed to the LiteLLM request body.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn task_image_data_urls_via_context_manager_includes_base64() {
    use ai_kanban_backend::ai::context_manager::ContextManager;
    use ai_kanban_backend::ai::litellm::LitellmClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Start a mock LiteLLM server
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "summary text"}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        })))
        .mount(&mock_server)
        .await;

    let (task_repo, comment_repo, attachment_repo) = setup_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Image Data URL Task".to_string(),
            description: None,
            project_path: "/tmp".to_string(),
        })
        .await
        .expect("create task");

    // Write a real PNG file to disk
    let tmp_img_path = format!("/tmp/test-pipeline-img2-{}.png", Uuid::new_v4());
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    std::fs::write(&tmp_img_path, png_bytes).expect("write test PNG");

    // Create attachment with real storage_path
    let att = make_attachment(&task.id, "photo.png", "image/png", &tmp_img_path);
    attachment_repo
        .create(&att)
        .await
        .expect("create attachment");

    let litellm = LitellmClient::new(&mock_server.uri(), "test-key", "test-model");
    let ctx = ContextManager::new(litellm, comment_repo, task_repo, attachment_repo);

    // summarize_session triggers task_image_data_urls internally
    let result = ctx
        .summarize_session(
            "sess-img",
            &task.id,
            &task.title,
            "in_progress",
            Some(10),
            50,
            20,
            &["did some work".to_string()],
            Some("result"),
        )
        .await;

    assert!(
        result.is_ok(),
        "summarize_session should succeed: {:?}",
        result.err()
    );

    // Check request body sent to mock server contained an image_url part
    let received = mock_server.received_requests().await.expect("get requests");
    assert!(!received.is_empty(), "LiteLLM should have been called");

    let body: serde_json::Value =
        serde_json::from_slice(&received[0].body).expect("parse request body");

    // The messages array should contain an image_url content part with base64 data
    let messages = body["messages"]
        .as_array()
        .expect("messages should be array");
    let has_image = messages.iter().any(|msg| {
        if let Some(content) = msg["content"].as_array() {
            content.iter().any(|part| {
                part["type"].as_str() == Some("image_url")
                    && part["image_url"]["url"]
                        .as_str()
                        .map(|u| u.starts_with("data:image/png;base64,"))
                        .unwrap_or(false)
            })
        } else {
            false
        }
    });

    assert!(
        has_image,
        "LiteLLM request should contain image_url part with base64 PNG data. Body: {}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );

    // Cleanup
    let _ = std::fs::remove_file(&tmp_img_path);
}
