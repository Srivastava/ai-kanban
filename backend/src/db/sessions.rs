use crate::models::{CreateSession, Session, UpdateSession};
use anyhow::{anyhow, Result};
use sqlx::SqlitePool;
use tracing::{debug, info, instrument};

#[derive(Clone)]
pub struct SessionRepository {
    pool: SqlitePool,
}

impl SessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self), fields(task_id = %create.task_id))]
    pub async fn create(&self, create: CreateSession) -> Result<Session> {
        let now = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, task_id, status, started_at)
            VALUES (?, ?, 'pending', ?)
            "#,
        )
        .bind(&id)
        .bind(&create.task_id)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        info!(session_id = %id, "Session created");

        Ok(Session {
            id,
            task_id: create.task_id,
            status: "pending".to_string(),
            started_at: now,
            ended_at: None,
            last_snapshot_id: None,
            error_message: None,
            claude_session_id: None,
        })
    }

    #[instrument(skip(self), fields(session_id = %id))]
    pub async fn find(&self, id: &str) -> Result<Session> {
        debug!(session_id = %id, "Finding session");
        let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("Session not found: {}", id))?;
        Ok(session)
    }

    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions ORDER BY started_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    #[instrument(skip(self), fields(task_id = %task_id))]
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<Session>> {
        debug!(task_id = %task_id, "Listing sessions for task");
        let sessions = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE task_id = ? ORDER BY started_at DESC"
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    #[instrument(skip(self), fields(session_id = %id))]
    pub async fn update(&self, id: &str, update: UpdateSession) -> Result<Session> {
        let mut session = self.find(id).await?;

        if let Some(status) = update.status {
            session.status = status;
        }
        if let Some(ended_at) = update.ended_at {
            session.ended_at = Some(ended_at);
        }
        if let Some(snapshot_id) = update.last_snapshot_id {
            session.last_snapshot_id = Some(snapshot_id);
        }
        if let Some(error) = update.error_message {
            session.error_message = Some(error);
        }
        if let Some(claude_session_id) = update.claude_session_id {
            session.claude_session_id = Some(claude_session_id);
        }

        sqlx::query(
            r#"
            UPDATE sessions
            SET status = ?, ended_at = ?, last_snapshot_id = ?, error_message = ?, claude_session_id = ?
            WHERE id = ?
            "#,
        )
        .bind(&session.status)
        .bind(session.ended_at.map(|t| t.to_rfc3339()))
        .bind(&session.last_snapshot_id)
        .bind(&session.error_message)
        .bind(&session.claude_session_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        info!(session_id = %id, status = %session.status, "Session updated");
        Ok(session)
    }

    #[instrument(skip(self), fields(session_id = %id))]
    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Session not found: {}", id));
        }

        info!(session_id = %id, "Session deleted");
        Ok(())
    }

    pub async fn list_by_status(&self, status: &str) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE status = ? ORDER BY started_at ASC"
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }
}
