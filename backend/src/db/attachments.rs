use crate::models::TaskAttachment;
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AttachmentRepository {
    pool: SqlitePool,
}

impl AttachmentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, attachment: &TaskAttachment) -> Result<TaskAttachment> {
        sqlx::query(
            "INSERT INTO task_attachments (id, task_id, filename, storage_path, mime_type, size_bytes, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&attachment.id)
        .bind(&attachment.task_id)
        .bind(&attachment.filename)
        .bind(&attachment.storage_path)
        .bind(&attachment.mime_type)
        .bind(attachment.size_bytes)
        .bind(attachment.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(attachment.clone())
    }

    pub async fn list_for_task(&self, task_id: &str) -> Result<Vec<TaskAttachment>> {
        let rows = sqlx::query_as::<_, TaskAttachment>(
            "SELECT * FROM task_attachments WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get(&self, id: &str) -> Result<Option<TaskAttachment>> {
        let row =
            sqlx::query_as::<_, TaskAttachment>("SELECT * FROM task_attachments WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM task_attachments WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
