use ai_kanban_backend::db::{create_pool, AttachmentRepository, TaskRepository};
use ai_kanban_backend::models::{CreateTask, TaskAttachment};
use chrono::Utc;
use uuid::Uuid;

async fn setup() -> (AttachmentRepository, TaskRepository) {
    let db_path = format!("/tmp/test-attach-{}.db", Uuid::new_v4());
    let pool = create_pool(&db_path).await.expect("Failed to create pool");
    let attachment_repo = AttachmentRepository::new(pool.clone());
    let task_repo = TaskRepository::new(pool.clone());
    (attachment_repo, task_repo)
}

fn make_attachment(task_id: &str) -> TaskAttachment {
    TaskAttachment {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        filename: "test.png".to_string(),
        storage_path: "/uploads/test.png".to_string(),
        mime_type: "image/png".to_string(),
        size_bytes: 1024,
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn create_stores_and_returns_attachment() {
    let (attachment_repo, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Task A".to_string(),
            description: None,
            project_path: "/tmp/proj".to_string(),
        })
        .await
        .unwrap();

    let attachment = make_attachment(&task.id);
    let returned = attachment_repo.create(&attachment).await.unwrap();

    assert_eq!(returned.id, attachment.id);
    assert_eq!(returned.task_id, task.id);
    assert_eq!(returned.filename, "test.png");
    assert_eq!(returned.mime_type, "image/png");
    assert_eq!(returned.storage_path, "/uploads/test.png");
}

#[tokio::test]
async fn list_for_task_returns_in_insertion_order() {
    let (attachment_repo, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Task B".to_string(),
            description: None,
            project_path: "/tmp/proj".to_string(),
        })
        .await
        .unwrap();

    let a1 = TaskAttachment {
        id: Uuid::new_v4().to_string(),
        task_id: task.id.clone(),
        filename: "first.png".to_string(),
        storage_path: "/uploads/first.png".to_string(),
        mime_type: "image/png".to_string(),
        size_bytes: 100,
        created_at: Utc::now(),
    };
    // Ensure second attachment has a later timestamp
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let a2 = TaskAttachment {
        id: Uuid::new_v4().to_string(),
        task_id: task.id.clone(),
        filename: "second.png".to_string(),
        storage_path: "/uploads/second.png".to_string(),
        mime_type: "image/png".to_string(),
        size_bytes: 200,
        created_at: Utc::now(),
    };

    attachment_repo.create(&a1).await.unwrap();
    attachment_repo.create(&a2).await.unwrap();

    let list = attachment_repo.list_for_task(&task.id).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].filename, "first.png");
    assert_eq!(list[1].filename, "second.png");
}

#[tokio::test]
async fn list_for_task_returns_empty_for_no_attachments() {
    let (attachment_repo, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Task C".to_string(),
            description: None,
            project_path: "/tmp/proj".to_string(),
        })
        .await
        .unwrap();

    let list = attachment_repo.list_for_task(&task.id).await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn get_returns_some_for_existing() {
    let (attachment_repo, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Task D".to_string(),
            description: None,
            project_path: "/tmp/proj".to_string(),
        })
        .await
        .unwrap();

    let attachment = make_attachment(&task.id);
    attachment_repo.create(&attachment).await.unwrap();

    let result = attachment_repo.get(&attachment.id).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, attachment.id);
}

#[tokio::test]
async fn get_returns_none_for_unknown() {
    let (attachment_repo, _) = setup().await;

    let result = attachment_repo.get("nonexistent-id").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn delete_removes_record() {
    let (attachment_repo, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Task E".to_string(),
            description: None,
            project_path: "/tmp/proj".to_string(),
        })
        .await
        .unwrap();

    let attachment = make_attachment(&task.id);
    attachment_repo.create(&attachment).await.unwrap();

    attachment_repo.delete(&attachment.id).await.unwrap();

    let result = attachment_repo.get(&attachment.id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn cascade_delete_removes_attachments() {
    let (attachment_repo, task_repo) = setup().await;

    let task = task_repo
        .create(CreateTask {
            title: "Task F".to_string(),
            description: None,
            project_path: "/tmp/proj".to_string(),
        })
        .await
        .unwrap();

    let a1 = make_attachment(&task.id);
    let a2 = make_attachment(&task.id);
    attachment_repo.create(&a1).await.unwrap();
    attachment_repo.create(&a2).await.unwrap();

    // Delete the parent task — cascade should remove attachments
    task_repo.delete(&task.id).await.unwrap();

    let list = attachment_repo.list_for_task(&task.id).await.unwrap();
    assert!(list.is_empty());

    let r1 = attachment_repo.get(&a1.id).await.unwrap();
    let r2 = attachment_repo.get(&a2.id).await.unwrap();
    assert!(r1.is_none());
    assert!(r2.is_none());
}
