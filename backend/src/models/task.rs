use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub stage: String,
    pub project_path: String,
    pub session_id: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: Option<String>,
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub stage: Option<String>,
    pub priority: Option<i32>,
}

impl Task {
    pub fn new(create: CreateTask) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: create.title,
            description: create.description,
            stage: "backlog".to_string(),
            project_path: create.project_path,
            session_id: None,
            priority: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stage {
    Backlog,
    Planning,
    Ready,
    InProgress,
    Review,
    Done,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stage::Backlog => "backlog",
            Stage::Planning => "planning",
            Stage::Ready => "ready",
            Stage::InProgress => "in_progress",
            Stage::Review => "review",
            Stage::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "backlog" => Some(Stage::Backlog),
            "planning" => Some(Stage::Planning),
            "ready" => Some(Stage::Ready),
            "in_progress" => Some(Stage::InProgress),
            "review" => Some(Stage::Review),
            "done" => Some(Stage::Done),
            _ => None,
        }
    }

    pub fn all() -> &'static [&'static str] {
        &["backlog", "planning", "ready", "in_progress", "review", "done"]
    }
}
