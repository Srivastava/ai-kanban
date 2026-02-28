use crate::models::{Comment, CommentWithReplies, CreateComment};
use anyhow::Result;
use sqlx::SqlitePool;
use tracing::{debug, info, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct CommentRepository {
    pool: SqlitePool,
}

impl CommentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self), fields(task_id = %task_id))]
    pub async fn list_for_task(&self, task_id: &str) -> Result<Vec<CommentWithReplies>> {
        debug!(task_id = %task_id, "Listing comments for task");

        // Get all comments for task
        let all_comments = sqlx::query_as::<_, Comment>(
            "SELECT * FROM task_comments WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        debug!(
            task_id = %task_id,
            count = all_comments.len(),
            "Comments retrieved"
        );

        // Separate into top-level and replies
        let mut top_level: Vec<Comment> = Vec::new();
        let mut replies_by_parent: std::collections::HashMap<String, Vec<Comment>> =
            std::collections::HashMap::new();

        for comment in all_comments {
            if let Some(parent_id) = &comment.parent_id {
                replies_by_parent
                    .entry(parent_id.clone())
                    .or_default()
                    .push(comment);
            } else {
                top_level.push(comment);
            }
        }

        // Build result with replies
        let result: Vec<CommentWithReplies> = top_level
            .into_iter()
            .map(|c| {
                let replies = replies_by_parent.remove(&c.id).unwrap_or_default();
                CommentWithReplies { comment: c, replies }
            })
            .collect();

        info!(
            task_id = %task_id,
            top_level_count = result.len(),
            "Comments organized"
        );
        Ok(result)
    }

    #[instrument(skip(self), fields(task_id = %task_id, author = %author))]
    pub async fn create(
        &self,
        task_id: &str,
        author: &str,
        data: CreateComment,
    ) -> Result<Comment> {
        let id = Uuid::new_v4().to_string();
        tracing::Span::current().record("comment_id", &id);

        info!(
            task_id = %task_id,
            author = %author,
            parent_id = ?data.parent_id,
            "Creating comment"
        );

        let comment = sqlx::query_as::<_, Comment>(
            r#"INSERT INTO task_comments (id, task_id, parent_id, author, content, created_at)
               VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(task_id)
        .bind(&data.parent_id)
        .bind(author)
        .bind(&data.content)
        .fetch_one(&self.pool)
        .await?;

        info!(comment_id = %comment.id, "Comment created successfully");
        Ok(comment)
    }

    #[instrument(skip(self), fields(comment_id = %id))]
    pub async fn delete(&self, id: &str) -> Result<()> {
        info!(comment_id = %id, "Deleting comment");

        let result = sqlx::query("DELETE FROM task_comments WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Comment not found: {}", id));
        }

        info!(comment_id = %id, "Comment deleted successfully");
        Ok(())
    }
}
