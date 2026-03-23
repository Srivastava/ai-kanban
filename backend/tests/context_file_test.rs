/// Tests for write_task_context_file
///
/// The function is pub(crate) in manager.rs but since tests in backend/tests/ are
/// external integration tests, we test via the public claude module export.
use ai_kanban_backend::claude::write_task_context_file;
use ai_kanban_backend::models::{CreateTask, Task};
use tokio::sync::broadcast;

fn make_task(title: &str) -> Task {
    Task::new(CreateTask {
        title: title.to_string(),
        description: None,
        project_path: "/tmp".to_string(),
    })
}

fn make_task_with_desc(title: &str, desc: &str) -> Task {
    Task::new(CreateTask {
        title: title.to_string(),
        description: Some(desc.to_string()),
        project_path: "/tmp".to_string(),
    })
}

fn make_broadcast() -> tokio::sync::broadcast::Sender<ai_kanban_backend::claude::ClaudeEvent> {
    let (tx, _) = broadcast::channel(16);
    tx
}

/// Returns a unique temp dir path for isolation between tests.
fn unique_tmp_dir(prefix: &str) -> String {
    let id = uuid::Uuid::new_v4();
    format!("/tmp/{}-{}", prefix, id)
}

#[test]
fn test_write_task_context_file_creates_directory() {
    let project_path = unique_tmp_dir("ctx-test-dir");
    // .claude/ dir should not exist yet
    let claude_dir = format!("{}/.claude", project_path);
    assert!(!std::path::Path::new(&claude_dir).exists());

    let task = make_task("My Task");
    let tx = make_broadcast();
    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-1");

    assert!(
        std::path::Path::new(&claude_dir).is_dir(),
        ".claude directory should have been created"
    );
    assert!(
        std::path::Path::new(&format!("{}/.claude/ai-kanban.md", project_path)).exists(),
        "ai-kanban.md file should exist"
    );
}

#[test]
fn test_write_task_context_file_contains_title() {
    let project_path = unique_tmp_dir("ctx-test-title");
    let task = make_task("Fix the login bug");
    let tx = make_broadcast();
    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path))
        .expect("file should exist");
    assert!(
        content.contains("Fix the login bug"),
        "file should contain the task title"
    );
    assert!(
        content.contains("# Task: Fix the login bug"),
        "title should be a markdown heading"
    );
}

#[test]
fn test_write_task_context_file_contains_description() {
    let project_path = unique_tmp_dir("ctx-test-desc");
    let task = make_task_with_desc("Add dark mode", "Support a dark color theme for all pages.");
    let tx = make_broadcast();
    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path))
        .expect("file should exist");
    assert!(
        content.contains("Support a dark color theme for all pages."),
        "file should contain the description"
    );
    assert!(
        content.contains("## Description"),
        "description should have a heading"
    );
}

#[test]
fn test_write_task_context_file_contains_plan() {
    let project_path = unique_tmp_dir("ctx-test-plan");

    let mut task = make_task("Refactor DB layer");
    task.instructions = Some("# Plan\n- Step 1: Extract repository\n- Step 2: Add tests".to_string());

    let tx = make_broadcast();
    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-1");

    let content = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path))
        .expect("file should exist");
    assert!(
        content.contains("## Implementation Plan"),
        "file should have Implementation Plan section"
    );
    assert!(
        content.contains("Step 1: Extract repository"),
        "file should contain plan steps"
    );
}

#[test]
fn test_write_task_context_file_updates_on_second_call() {
    let project_path = unique_tmp_dir("ctx-test-overwrite");
    let tx = make_broadcast();

    // First write
    let task1 = make_task("Original Task Title");
    write_task_context_file(&project_path, &task1, &[], &[], &tx, "session-1");

    let content1 = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path))
        .expect("file should exist after first write");
    assert!(content1.contains("Original Task Title"));

    // Second write should overwrite (not append)
    let task2 = make_task("Updated Task Title");
    write_task_context_file(&project_path, &task2, &[], &[], &tx, "session-2");

    let content2 = std::fs::read_to_string(format!("{}/.claude/ai-kanban.md", project_path))
        .expect("file should exist after second write");
    assert!(
        content2.contains("Updated Task Title"),
        "second write should have updated the title"
    );
    assert!(
        !content2.contains("Original Task Title"),
        "file should not contain old title (must be overwritten, not appended)"
    );
}

#[test]
fn test_write_task_context_file_emits_event_on_success() {
    let project_path = unique_tmp_dir("ctx-test-event");
    let (tx, mut rx) = broadcast::channel(16);
    let task = make_task("Event Test Task");
    write_task_context_file(&project_path, &task, &[], &[], &tx, "session-abc");

    // Should have received a ContextFileUpdated event
    match rx.try_recv() {
        Ok(ai_kanban_backend::claude::ClaudeEvent::ContextFileUpdated { session_id, task_id }) => {
            assert_eq!(session_id, "session-abc");
            assert_eq!(task_id, task.id);
        }
        other => panic!("Expected ContextFileUpdated event, got {:?}", other),
    }
}

#[test]
fn test_write_task_context_file_no_event_on_invalid_path() {
    // Use a path that cannot be created (root-owned path)
    let project_path = "/proc/sys/kernel/cannot_create_here";
    let (tx, mut rx) = broadcast::channel(16);
    let task = make_task("Should Fail Task");
    write_task_context_file(project_path, &task, &[], &[], &tx, "session-fail");

    // Should NOT have received a ContextFileUpdated event (write failed)
    match rx.try_recv() {
        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
            // Correct — no event emitted on failure
        }
        Ok(event) => panic!("Should not have emitted event on failure, got {:?}", event),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}
