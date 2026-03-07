use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OtelLog {
    pub id: i64,
    pub event_name: String,
    pub body: Option<String>,
    pub severity_text: Option<String>,
    pub severity_number: Option<i64>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub claude_session_id: String,
    pub attributes: String, // JSON text
    pub otel_timestamp: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateOtelLog {
    pub event_name: String,
    pub body: Option<String>,
    pub severity_text: Option<String>,
    pub severity_number: Option<i64>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub claude_session_id: String,
    pub attributes: serde_json::Value,
    pub otel_timestamp: i64,
}
