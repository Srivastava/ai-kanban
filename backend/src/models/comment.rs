use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: String,
    pub task_id: String,
    pub parent_id: Option<String>,
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateComment {
    pub content: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentWithReplies {
    #[serde(flatten)]
    pub comment: Comment,
    pub replies: Vec<Comment>,
}

impl Comment {
    pub fn new(task_id: &str, author: &str, create: CreateComment) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            parent_id: create.parent_id,
            author: author.to_string(),
            content: create.content,
            created_at: Utc::now(),
        }
    }
}
