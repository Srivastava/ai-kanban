use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskAttachment {
    pub id: String,
    pub task_id: String,
    pub filename: String,
    pub storage_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}
