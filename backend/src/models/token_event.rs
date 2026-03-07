use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single token event from Claude's JSONL stdout stream
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TokenEvent {
    pub id: i64,
    pub session_id: String,
    pub task_id: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub file_ext: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: Option<String>,
    pub sequence_no: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenEvent {
    pub session_id: String,
    pub task_id: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub file_ext: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub model: Option<String>,
    pub sequence_no: Option<i64>,
}

/// Per-session project metrics (captured at start, updated during run)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionMetrics {
    pub session_id: String,
    pub project_files: i64,
    pub project_loc: i64,
    pub lines_written: i64,
    pub lines_deleted: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
