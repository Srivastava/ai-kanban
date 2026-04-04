use crate::models::{CreateTask, Task, UpdateTask};
use anyhow::{anyhow, Result};
use sqlx::SqlitePool;
use tracing::{debug, info, instrument};

#[derive(Clone)]
pub struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self), fields(stage = ?stage))]
    pub async fn list(&self, stage: Option<&str>) -> Result<Vec<Task>> {
        debug!(stage = ?stage, "Listing tasks");
        let tasks =
            match stage {
                Some(s) => sqlx::query_as::<_, Task>(
                    "SELECT * FROM tasks WHERE stage = ? ORDER BY priority DESC, created_at ASC",
                )
                .bind(s)
                .fetch_all(&self.pool)
                .await?,
                None => {
                    sqlx::query_as::<_, Task>(
                        "SELECT * FROM tasks ORDER BY stage, priority DESC, created_at ASC",
                    )
                    .fetch_all(&self.pool)
                    .await?
                }
            };
        info!(count = tasks.len(), stage = ?stage, "Tasks retrieved");
        Ok(tasks)
    }

    #[instrument(skip(self), fields(task_id = %id))]
    pub async fn find(&self, id: &str) -> Result<Task> {
        debug!(task_id = %id, "Finding task");
        let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;
        debug!(task_id = %id, stage = %task.stage, "Task found");
        Ok(task)
    }

    #[instrument(skip(self), fields(task_id))]
    pub async fn create(&self, create: CreateTask) -> Result<Task> {
        let task = Task::new(create);
        tracing::Span::current().record("task_id", &task.id);

        info!(
            task_id = %task.id,
            title = %task.title,
            project_path = %task.project_path,
            "Creating new task"
        );

        sqlx::query(
            r#"
            INSERT INTO tasks (id, title, description, stage, project_path, session_id, priority, context, compressed_context, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(&task.stage)
        .bind(&task.project_path)
        .bind(&task.session_id)
        .bind(task.priority)
        .bind(&task.context)
        .bind(&task.compressed_context)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        info!(task_id = %task.id, "Task created successfully");
        Ok(task)
    }

    #[instrument(skip(self), fields(task_id = %id))]
    pub async fn update(&self, id: &str, update: UpdateTask) -> Result<Task> {
        info!(
            task_id = %id,
            title = ?update.title,
            stage = ?update.stage,
            priority = ?update.priority,
            "Updating task"
        );

        let mut task = self.find(id).await?;

        if let Some(title) = update.title {
            task.title = title;
        }
        if let Some(description) = update.description {
            task.description = Some(description);
        }
        if let Some(instructions) = update.instructions {
            task.instructions = instructions;
        }
        if let Some(stage) = update.stage {
            task.stage = stage;
        }
        if let Some(priority) = update.priority {
            task.priority = priority;
        }
        if let Some(context) = update.context {
            task.context = Some(context);
        }
        if let Some(session_id) = update.session_id {
            task.session_id = Some(session_id);
        }
        task.updated_at = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE tasks
            SET title = ?, description = ?, instructions = ?, stage = ?, priority = ?, context = ?, session_id = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&task.title)
        .bind(&task.description)
        .bind(&task.instructions)
        .bind(&task.stage)
        .bind(task.priority)
        .bind(&task.context)
        .bind(&task.session_id)
        .bind(task.updated_at.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        info!(task_id = %id, new_stage = %task.stage, "Task updated");
        Ok(task)
    }

    #[instrument(skip(self), fields(task_id = %id))]
    pub async fn delete(&self, id: &str) -> Result<()> {
        info!(task_id = %id, "Deleting task with related records");

        // First verify task exists
        let task = self.find(id).await?;

        // All deletions run inside a single transaction so a crash mid-delete
        // can't leave the DB in a partially-deleted state.
        let mut tx = self.pool.begin().await?;

        // Delete tables that have FK references to tasks without ON DELETE CASCADE.
        // (Tables with ON DELETE CASCADE — task_comments, token_events, task_attachments —
        // are handled automatically by SQLite's FK enforcement.)
        sqlx::query("DELETE FROM otel_metrics WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM stage_history WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM token_usage WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM snapshots WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;

        // Then delete the task (triggers ON DELETE CASCADE for remaining child tables)
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Task not found: {}", id));
        }

        tx.commit().await?;

        info!(task_id = %id, "Task deleted successfully");
        Ok(())
    }

    pub async fn update_compressed_context(
        &self,
        id: &str,
        compressed_context: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE tasks SET compressed_context = ?, updated_at = ? WHERE id = ?")
            .bind(compressed_context)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[instrument(skip(self), fields(task_id = %id))]
    pub async fn move_to_stage(&self, id: &str, new_stage: &str) -> Result<Task> {
        let old_task = self.find(id).await?;
        let old_stage = old_task.stage.clone();

        info!(
            task_id = %id,
            from_stage = %old_stage,
            to_stage = %new_stage,
            "Moving task to new stage"
        );

        let task = self
            .update(
                id,
                UpdateTask {
                    stage: Some(new_stage.to_string()),
                    ..Default::default()
                },
            )
            .await?;

        // Record stage history
        sqlx::query("INSERT INTO stage_history (task_id, from_stage, to_stage) VALUES (?, ?, ?)")
            .bind(id)
            .bind(&old_stage)
            .bind(new_stage)
            .execute(&self.pool)
            .await?;

        info!(
            task_id = %id,
            from_stage = %old_stage,
            to_stage = %new_stage,
            "Task moved successfully"
        );
        Ok(task)
    }
}
