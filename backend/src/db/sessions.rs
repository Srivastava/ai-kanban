use crate::models::{CreateSession, Session, SessionDetail, UpdateSession};
use anyhow::{anyhow, Result};
use sqlx::{Row, SqlitePool};
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

    /// List sessions, optionally filtered by one or more statuses, newest first.
    pub async fn list_recent(&self, statuses: &[&str], limit: i64) -> Result<Vec<Session>> {
        if statuses.is_empty() {
            let rows = sqlx::query_as::<_, Session>(
                "SELECT * FROM sessions ORDER BY started_at DESC LIMIT ?"
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
            return Ok(rows);
        }

        // Build parameterised IN clause — sqlx doesn't support dynamic IN natively
        let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT * FROM sessions WHERE status IN ({}) ORDER BY started_at DESC LIMIT ?",
            placeholders
        );
        let mut q = sqlx::query_as::<_, Session>(&sql);
        for s in statuses {
            q = q.bind(*s);
        }
        q = q.bind(limit);
        Ok(q.fetch_all(&self.pool).await?)
    }

    pub async fn find_by_claude_session_id(&self, claude_session_id: &str) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE claude_session_id = ? LIMIT 1"
        )
        .bind(claude_session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_by_task_with_tokens(&self, task_id: &str) -> Result<Vec<SessionDetail>> {
        let rows = sqlx::query(
            r#"
            SELECT
                s.id,
                s.task_id,
                s.status,
                s.started_at,
                s.ended_at,
                s.claude_session_id,
                s.error_message,
                COALESCE(SUM(te.input_tokens), 0) as input_tokens,
                COALESCE(SUM(te.output_tokens), 0) as output_tokens,
                COALESCE(SUM(te.input_tokens + te.output_tokens), 0) as total_tokens
            FROM sessions s
            LEFT JOIN token_events te ON te.session_id = s.id
            WHERE s.task_id = ?
            GROUP BY s.id
            ORDER BY s.started_at DESC
            "#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        let mut details = Vec::new();
        for row in &rows {
            let started_at: String = row.get("started_at");
            let ended_at: Option<String> = row.get("ended_at");
            let duration_secs = ended_at.as_deref().and_then(|end| {
                let s = chrono::DateTime::parse_from_rfc3339(&started_at).ok()?;
                let e = chrono::DateTime::parse_from_rfc3339(end).ok()?;
                Some((e - s).num_seconds())
            });
            details.push(SessionDetail {
                id: row.get("id"),
                task_id: row.get("task_id"),
                status: row.get("status"),
                started_at,
                ended_at,
                claude_session_id: row.get("claude_session_id"),
                error_message: row.get("error_message"),
                duration_secs,
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                total_tokens: row.get("total_tokens"),
            });
        }
        Ok(details)
    }
}
