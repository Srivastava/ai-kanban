use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub task_id: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_snapshot_id: Option<String>,
    pub error_message: Option<String>,
    pub claude_session_id: Option<String>,
    pub peak_context_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Pending,
    Running,
    Stopped,
    Completed,
    Failed,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Pending => "pending",
            SessionStatus::Running => "running",
            SessionStatus::Stopped => "stopped",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(SessionStatus::Pending),
            "running" => Some(SessionStatus::Running),
            "stopped" => Some(SessionStatus::Stopped),
            "completed" => Some(SessionStatus::Completed),
            "failed" => Some(SessionStatus::Failed),
            _ => None,
        }
    }

    pub fn all() -> &'static [&'static str] {
        &["pending", "running", "stopped", "completed", "failed"]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSession {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSession {
    pub status: Option<String>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_snapshot_id: Option<String>,
    pub error_message: Option<String>,
    /// `None` = do not touch the column.
    /// `Some(Some(id))` = set to the given session ID.
    /// `Some(None)` = clear to NULL (used after context compression so the next
    /// `continue_session` starts fresh and actually uses the compressed context
    /// rather than re-inheriting the full 150 K+ token history via `--resume`).
    pub claude_session_id: Option<Option<String>>,
    pub peak_context_tokens: Option<i64>,
}
