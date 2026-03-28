use serde::{Deserialize, Serialize};

/// Messages from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe_task")]
    SubscribeTask { task_id: String },

    #[serde(rename = "subscribe_session")]
    SubscribeSession { session_id: String },

    #[serde(rename = "ping")]
    Ping,
}

/// Messages from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "task_updated")]
    TaskUpdated { task: serde_json::Value },

    #[serde(rename = "session_output")]
    SessionOutput {
        session_id: String,
        output: String,
        is_error: bool,
    },

    #[serde(rename = "session_status")]
    SessionStatus { session_id: String, status: String },

    #[serde(rename = "session_heartbeat")]
    SessionHeartbeat {
        session_id: String,
        elapsed_secs: u64,
    },

    #[serde(rename = "session_id_assigned")]
    SessionIdAssigned {
        session_id: String,
        claude_session_id: String,
    },

    #[serde(rename = "rate_limited")]
    RateLimited {
        session_id: String,
        task_id: String,
        reset_at: String, // ISO 8601 string
    },

    #[serde(rename = "stage_context_set")]
    StageContextSet {
        session_id: String,
        task_id: String,
        mode: String,
    },

    #[serde(rename = "context_file_updated")]
    ContextFileUpdated { session_id: String, task_id: String },

    #[serde(rename = "plan_created")]
    PlanCreated {
        session_id: String,
        task_id: String,
        preview: String,
    },

    #[serde(rename = "enrichment_started")]
    EnrichmentStarted { task_id: String },

    #[serde(rename = "enrichment_completed")]
    EnrichmentCompleted { task_id: String },

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "subscribed")]
    Subscribed { topic: String },
}

impl ServerMessage {
    pub fn session_output(session_id: String, output: String, is_error: bool) -> Self {
        ServerMessage::SessionOutput {
            session_id,
            output,
            is_error,
        }
    }

    pub fn session_status(session_id: String, status: String) -> Self {
        ServerMessage::SessionStatus { session_id, status }
    }

    pub fn session_heartbeat(session_id: String, elapsed_secs: u64) -> Self {
        ServerMessage::SessionHeartbeat {
            session_id,
            elapsed_secs,
        }
    }

    pub fn pong() -> Self {
        ServerMessage::Pong
    }

    pub fn error(message: impl Into<String>) -> Self {
        ServerMessage::Error {
            message: message.into(),
        }
    }

    pub fn subscribed(topic: impl Into<String>) -> Self {
        ServerMessage::Subscribed {
            topic: topic.into(),
        }
    }
}
