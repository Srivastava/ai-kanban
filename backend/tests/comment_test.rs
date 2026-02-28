use ai_kanban_backend::db::{create_pool, CommentRepository, TaskRepository};
use ai_kanban_backend::models::{CreateComment, CreateTask};

async fn setup_test_db() -> (TaskRepository, CommentRepository) {
    let db_path = format!("/tmp/test-comment-{}.db", uuid::Uuid::new_v4());
    let pool = create_pool(&db_path)
        .await
        .expect("Failed to create pool");
    (
        TaskRepository::new(pool.clone()),
        CommentRepository::new(pool),
    )
}

// ==================== Comment Repository Tests ====================

#[tokio::test]
async fn test_comment_create() {
    let (task_repo, comment_repo) = setup_test_db().await;

    // First create a task
    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create a comment
    let comment = comment_repo
        .create(&task.id, "user", CreateComment {
            content: "This is a test comment".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    assert!(!comment.id.is_empty());
    assert_eq!(comment.task_id, task.id);
    assert_eq!(comment.author, "user");
    assert_eq!(comment.content, "This is a test comment");
    assert!(comment.parent_id.is_none());
}

#[tokio::test]
async fn test_comment_create_with_parent() {
    let (task_repo, comment_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create parent comment
    let parent = comment_repo
        .create(&task.id, "user", CreateComment {
            content: "Parent comment".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    // Create reply
    let reply = comment_repo
        .create(&task.id, "claude", CreateComment {
            content: "Reply comment".to_string(),
            parent_id: Some(parent.id.clone()),
        })
        .await
        .unwrap();

    assert!(reply.parent_id.is_some());
    assert_eq!(reply.parent_id.unwrap(), parent.id);
}

#[tokio::test]
async fn test_comment_list_for_task() {
    let (task_repo, comment_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create multiple comments
    comment_repo
        .create(&task.id, "user", CreateComment {
            content: "Comment 1".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    comment_repo
        .create(&task.id, "user", CreateComment {
            content: "Comment 2".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    let comments = comment_repo.list_for_task(&task.id).await.unwrap();
    assert_eq!(comments.len(), 2);
}

#[tokio::test]
async fn test_comment_with_reply() {
    let (task_repo, comment_repo) = setup_test_db().await;

    // Create task
    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create parent comment
    let parent = comment_repo
        .create(&task.id, "user", CreateComment {
            content: "Parent comment".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    // Create two replies
    comment_repo
        .create(&task.id, "claude", CreateComment {
            content: "Reply 1".to_string(),
            parent_id: Some(parent.id.clone()),
        })
        .await
        .unwrap();

    comment_repo
        .create(&task.id, "user", CreateComment {
            content: "Reply 2".to_string(),
            parent_id: Some(parent.id.clone()),
        })
        .await
        .unwrap();

    // List comments and verify structure
    let comments = comment_repo.list_for_task(&task.id).await.unwrap();

    assert_eq!(comments.len(), 1); // One top-level comment
    assert_eq!(comments[0].comment.id, parent.id);
    assert_eq!(comments[0].replies.len(), 2); // Two replies
}

#[tokio::test]
async fn test_comment_list_empty() {
    let (task_repo, comment_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    let comments = comment_repo.list_for_task(&task.id).await.unwrap();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_comment_delete() {
    let (task_repo, comment_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    let comment = comment_repo
        .create(&task.id, "user", CreateComment {
            content: "To be deleted".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    // Delete the comment
    comment_repo.delete(&comment.id).await.unwrap();

    // Verify it's gone
    let comments = comment_repo.list_for_task(&task.id).await.unwrap();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_comment_delete_not_found() {
    let (_, comment_repo) = setup_test_db().await;

    let result = comment_repo.delete("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_comments_only_for_specific_task() {
    let (task_repo, comment_repo) = setup_test_db().await;

    let task1 = task_repo
        .create(CreateTask {
            title: "Task 1".to_string(),
            description: None,
            project_path: "/tmp/test1".to_string(),
        })
        .await
        .unwrap();

    let task2 = task_repo
        .create(CreateTask {
            title: "Task 2".to_string(),
            description: None,
            project_path: "/tmp/test2".to_string(),
        })
        .await
        .unwrap();

    // Create comments for both tasks
    comment_repo
        .create(&task1.id, "user", CreateComment {
            content: "Task 1 comment".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    comment_repo
        .create(&task2.id, "user", CreateComment {
            content: "Task 2 comment".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    // Verify each task only sees its own comments
    let task1_comments = comment_repo.list_for_task(&task1.id).await.unwrap();
    assert_eq!(task1_comments.len(), 1);
    assert_eq!(task1_comments[0].comment.content, "Task 1 comment");

    let task2_comments = comment_repo.list_for_task(&task2.id).await.unwrap();
    assert_eq!(task2_comments.len(), 1);
    assert_eq!(task2_comments[0].comment.content, "Task 2 comment");
}

#[tokio::test]
async fn test_comment_ordering() {
    let (task_repo, comment_repo) = setup_test_db().await;

    let task = task_repo
        .create(CreateTask {
            title: "Test Task".to_string(),
            description: None,
            project_path: "/tmp/test".to_string(),
        })
        .await
        .unwrap();

    // Create multiple comments with small delays to ensure ordering
    let c1 = comment_repo
        .create(&task.id, "user", CreateComment {
            content: "First".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let c2 = comment_repo
        .create(&task.id, "user", CreateComment {
            content: "Second".to_string(),
            parent_id: None,
        })
        .await
        .unwrap();

    let comments = comment_repo.list_for_task(&task.id).await.unwrap();
    assert_eq!(comments.len(), 2);
    // Comments should be ordered by created_at ASC
    assert_eq!(comments[0].comment.id, c1.id);
    assert_eq!(comments[1].comment.id, c2.id);
}
