use crate::ai::litellm::{ChatMessage, LitellmClient};
use crate::db::CommentRepository;
use crate::models::CreateComment;
use anyhow::Result;
use tracing::{info, warn};

pub struct ContextManager {
    litellm: LitellmClient,
    comment_repo: CommentRepository,
}

impl ContextManager {
    pub fn new(litellm: LitellmClient, comment_repo: CommentRepository) -> Self {
        Self { litellm, comment_repo }
    }

    /// Summarize a completed session using LiteLLM and post the result as a claude comment.
    ///
    /// `display_lines` — human-readable activity lines collected from Claude's stdout
    /// (tool uses, text output) during the session.
    pub async fn summarize_session(
        &self,
        session_id: &str,
        task_id: &str,
        task_title: &str,
        display_lines: &[String],
        result_text: Option<&str>,
    ) -> Result<()> {
        if display_lines.is_empty() && result_text.map_or(true, |r| r.is_empty()) {
            info!(session_id = %session_id, "No session output — skipping summarization");
            return Ok(());
        }

        // Extract file-edit lines for the "files modified" section
        let file_edits: Vec<&str> = display_lines.iter()
            .filter(|l| l.contains("✏️") || l.starts_with("📖"))
            .map(|s| s.as_str())
            .collect();

        // Truncate activity to avoid blowing the model's context window
        let activity_lines: Vec<&str> = display_lines.iter()
            .take(300)
            .map(|s| s.as_str())
            .collect();
        let activity = activity_lines.join("\n");

        let result_section = match result_text {
            Some(r) if !r.is_empty() => {
                let preview = if r.len() > 600 { &r[..600] } else { r };
                format!("\n\nFinal output (preview):\n{}", preview)
            }
            _ => String::new(),
        };

        let user_content = format!(
            "Task: {task_title}\n\nSession activity ({count} events, showing first 300):\n{activity}{result_section}\n\nWrite a concise 3-5 sentence summary covering: what was accomplished, key files changed, and any important decisions or issues.",
            task_title = task_title,
            count = display_lines.len(),
            activity = activity,
            result_section = result_section,
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a concise project assistant. Summarize Claude Code session activity in plain language. Be specific about file names and what changed. Avoid vague language.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        info!(
            session_id = %session_id,
            task_id = %task_id,
            display_lines = display_lines.len(),
            model = %self.litellm.model,
            "Requesting session summary from LiteLLM"
        );

        match self.litellm.complete(messages).await {
            Ok(result) => {
                info!(
                    session_id = %session_id,
                    task_id = %task_id,
                    input_tokens = result.input_tokens,
                    output_tokens = result.output_tokens,
                    "Session summarization complete"
                );

                let files_section = if !file_edits.is_empty() {
                    let unique_edits: Vec<&str> = {
                        let mut seen = std::collections::HashSet::new();
                        file_edits.into_iter().filter(|l| seen.insert(*l)).collect()
                    };
                    format!(
                        "\n\n**Files touched ({}):**\n{}",
                        unique_edits.len(),
                        unique_edits.join("\n")
                    )
                } else {
                    String::new()
                };

                let comment_content = format!(
                    "**Session Summary** *(via LiteLLM · {} in / {} out tokens)*\n\n{}{}",
                    result.input_tokens,
                    result.output_tokens,
                    result.content.trim(),
                    files_section,
                );

                match self.comment_repo.create(
                    task_id,
                    "claude",
                    CreateComment { content: comment_content, parent_id: None },
                ).await {
                    Ok(comment) => info!(
                        session_id = %session_id,
                        task_id = %task_id,
                        comment_id = %comment.id,
                        "Posted session summary as update comment"
                    ),
                    Err(e) => warn!(
                        session_id = %session_id,
                        task_id = %task_id,
                        error = %e,
                        "Failed to post session summary comment"
                    ),
                }
            }
            Err(e) => {
                warn!(
                    session_id = %session_id,
                    task_id = %task_id,
                    error = %e,
                    "LiteLLM summarization failed — session summary skipped"
                );
            }
        }

        Ok(())
    }
}
