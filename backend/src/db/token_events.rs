use crate::models::{CreateTokenEvent, TokenEvent};
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct TokenEventRepository {
    pool: SqlitePool,
}

impl TokenEventRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create(&self, create: CreateTokenEvent) -> Result<TokenEvent> {
        let now = chrono::Utc::now();
        let seq = create.sequence_no.unwrap_or(0);

        let result = sqlx::query(
            r#"
            INSERT INTO token_events
                (session_id, task_id, event_type, tool_name, file_ext,
                 input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                 model, sequence_no, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&create.session_id)
        .bind(&create.task_id)
        .bind(&create.event_type)
        .bind(&create.tool_name)
        .bind(&create.file_ext)
        .bind(create.input_tokens)
        .bind(create.output_tokens)
        .bind(create.cache_read_tokens)
        .bind(create.cache_creation_tokens)
        .bind(&create.model)
        .bind(seq)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(TokenEvent {
            id: result.last_insert_rowid(),
            session_id: create.session_id,
            task_id: create.task_id,
            event_type: create.event_type,
            tool_name: create.tool_name,
            file_ext: create.file_ext,
            input_tokens: create.input_tokens,
            output_tokens: create.output_tokens,
            model: create.model,
            sequence_no: seq,
            timestamp: now,
        })
    }

    pub async fn list_by_session(&self, session_id: &str) -> Result<Vec<TokenEvent>> {
        let events = sqlx::query_as::<_, TokenEvent>(
            "SELECT * FROM token_events WHERE session_id = ? ORDER BY sequence_no ASC, id ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(events)
    }

    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<TokenEvent>> {
        let events = sqlx::query_as::<_, TokenEvent>(
            "SELECT * FROM token_events WHERE task_id = ? ORDER BY timestamp ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(events)
    }

    pub async fn create_batch(&self, events: Vec<CreateTokenEvent>) -> Result<()> {
        for event in events {
            self.create(event).await?;
        }
        Ok(())
    }
}
