use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Log {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub target: Option<String>,
    pub source: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLog {
    pub level: String,
    pub message: String,
    pub target: Option<String>,
    pub source: Option<String>,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogFilter {
    pub level: Option<String>,
    pub source: Option<String>,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl Log {
    pub fn new(create: CreateLog) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Will be set by database
            timestamp: now,
            level: create.level,
            message: create.message,
            target: create.target,
            source: create.source.unwrap_or_else(|| "backend".to_string()),
            task_id: create.task_id,
            session_id: create.session_id,
            metadata: create.metadata.map(|m| m.to_string()),
            created_at: now,
        }
    }
}

pub fn level_to_str(level: &str) -> &'static str {
    match level.to_lowercase().as_str() {
        "debug" => "DEBUG",
        "info" => "INFO",
        "warn" | "warning" => "WARN",
        "error" => "ERROR",
        _ => "INFO",
    }
}
