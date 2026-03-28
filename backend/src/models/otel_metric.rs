use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OtelMetric {
    pub id: i64,
    pub metric_name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub claude_session_id: String,
    pub attributes: String, // JSON text
    pub otel_timestamp: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOtelMetric {
    pub metric_name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub claude_session_id: String,
    pub attributes: serde_json::Value,
    pub otel_timestamp: i64,
}

/// Aggregated dev-activity row for Analytics — one row per task (all sessions summed)
/// Sources: token data from token_events (well-correlated); lines/cost from otel_metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevActivityRow {
    pub task_id: String,
    pub task_title: String,
    pub session_count: i64,
    pub lines_added: f64,
    pub lines_deleted: f64,
    pub input_tokens: f64,
    pub output_tokens: f64,
    pub cache_creation_tokens: f64,
    pub cache_read_tokens: f64,
    pub cost_usd: f64,
}
