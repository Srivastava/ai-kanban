use crate::models::{CreateTask, Task, UpdateTask};
use anyhow::{anyhow, Result};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, stage: Option<&str>) -> Result<Vec<Task>> {
        let tasks = match stage {
            Some(s) => {
                sqlx::query_as::<_, Task>(
                    "SELECT * FROM tasks WHERE stage = ? ORDER BY priority DESC, created_at ASC"
                )
                .bind(s)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, Task>(
                    "SELECT * FROM tasks ORDER BY stage, priority DESC, created_at ASC"
                )
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(tasks)
    }

    pub async fn find(&self, id: &str) -> Result<Task> {
        let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;
        Ok(task)
    }

    pub async fn create(&self, create: CreateTask) -> Result<Task> {
        let task = Task::new(create);
        sqlx::query(
            r#"
            INSERT INTO tasks (id, title, description, stage, project_path, session_id, priority, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(&task.stage)
        .bind(&task.project_path)
        .bind(&task.session_id)
        .bind(task.priority)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(task)
    }

    pub async fn update(&self, id: &str, update: UpdateTask) -> Result<Task> {
        let mut task = self.find(id).await?;

        if let Some(title) = update.title {
            task.title = title;
        }
        if let Some(description) = update.description {
            task.description = Some(description);
        }
        if let Some(stage) = update.stage {
            task.stage = stage;
        }
        if let Some(priority) = update.priority {
            task.priority = priority;
        }
        task.updated_at = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE tasks
            SET title = ?, description = ?, stage = ?, priority = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&task.title)
        .bind(&task.description)
        .bind(&task.stage)
        .bind(task.priority)
        .bind(task.updated_at.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(task)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Task not found: {}", id));
        }

        Ok(())
    }

    pub async fn move_to_stage(&self, id: &str, new_stage: &str) -> Result<Task> {
        let old_task = self.find(id).await?;
        let old_stage = old_task.stage.clone();

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
        sqlx::query(
            "INSERT INTO stage_history (task_id, from_stage, to_stage) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(&old_stage)
        .bind(new_stage)
        .execute(&self.pool)
        .await?;

        Ok(task)
    }
}
