use crate::ai::litellm::{ChatMessage, LitellmClient};
use crate::db::{CommentRepository, TaskRepository};
use crate::models::CreateComment;
use anyhow::Result;
use tracing::{info, warn};

pub struct ContextManager {
    litellm: LitellmClient,
    comment_repo: CommentRepository,
    task_repo: TaskRepository,
}

impl ContextManager {
    pub fn new(litellm: LitellmClient, comment_repo: CommentRepository, task_repo: TaskRepository) -> Self {
        Self { litellm, comment_repo, task_repo }
    }

    /// Summarize a completed session using LiteLLM and post the result as a litellm comment.
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

        let file_edits: Vec<&str> = display_lines.iter()
            .filter(|l| l.contains("✏️") || l.starts_with("📖"))
            .map(|s| s.as_str())
            .collect();

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
                    "litellm",
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

    /// Compress session context when token usage approaches the threshold.
    /// Stores a condensed context summary on the task for use in future sessions.
    pub async fn compress_context(
        &self,
        session_id: &str,
        task_id: &str,
        task_title: &str,
        display_lines: &[String],
        result_text: Option<&str>,
    ) -> Result<()> {
        let activity = display_lines.iter().take(400).cloned().collect::<Vec<_>>().join("\n");
        let result_section = result_text
            .filter(|r| !r.is_empty())
            .map(|r| {
                let preview = if r.len() > 800 { &r[..800] } else { r };
                format!("\n\nFinal output:\n{preview}")
            })
            .unwrap_or_default();

        let user_content = format!(
            "Task: {task_title}\n\nSession activity:\n{activity}{result_section}\n\n\
            Create a structured context summary (under 600 words) for the next Claude session. Include:\n\
            1. What was accomplished\n\
            2. Key decisions and rationale\n\
            3. Files modified and why\n\
            4. Current state of the codebase\n\
            5. What remains to be done\n\
            6. Any important constraints or gotchas discovered",
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are compressing a Claude Code session into a context handoff for the next session. Be specific, structured, and concise. Focus on information Claude needs to continue effectively.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        info!(
            session_id = %session_id,
            task_id = %task_id,
            model = %self.litellm.model,
            "Compressing session context with LiteLLM"
        );

        match self.litellm.complete(messages).await {
            Ok(result) => {
                let compressed = format!(
                    "*(Context compressed by LiteLLM — {} in / {} out tokens)*\n\n{}",
                    result.input_tokens, result.output_tokens, result.content.trim()
                );
                match self.task_repo.update_compressed_context(task_id, &compressed).await {
                    Ok(()) => info!(
                        session_id = %session_id,
                        task_id = %task_id,
                        "Stored compressed context on task"
                    ),
                    Err(e) => warn!(
                        session_id = %session_id,
                        task_id = %task_id,
                        error = %e,
                        "Failed to store compressed context"
                    ),
                }
            }
            Err(e) => {
                warn!(
                    session_id = %session_id,
                    task_id = %task_id,
                    error = %e,
                    "LiteLLM context compression failed"
                );
            }
        }

        Ok(())
    }

    /// Generate a condensed briefing from a long conversation history.
    /// Used before continue_session to reduce token usage on the next Claude turn.
    pub async fn generate_briefing(
        &self,
        task_title: &str,
        conversation_context: &str,
    ) -> Result<String> {
        let user_content = format!(
            "Task: {task_title}\n\nConversation history:\n{conversation_context}\n\n\
            Compress this into a brief context summary (under 400 words) for the next Claude session. Keep:\n\
            - Key decisions and outcomes\n\
            - Current state\n\
            - What needs to happen next\n\
            - Any blockers or constraints",
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are condensing a task conversation history into a minimal context summary. Be precise and omit redundant information.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        info!(task_title = %task_title, model = %self.litellm.model, "Generating pre-session briefing");

        match self.litellm.complete(messages).await {
            Ok(result) => {
                info!(
                    input_tokens = result.input_tokens,
                    output_tokens = result.output_tokens,
                    "Pre-session briefing generated"
                );
                Ok(format!(
                    "*(Briefing compressed by LiteLLM)*\n\n{}",
                    result.content.trim()
                ))
            }
            Err(e) => {
                warn!(error = %e, "Pre-session briefing failed — using original context");
                Err(e)
            }
        }
    }

    /// Enrich a terse task description before the first Claude session.
    /// Updates the task description in DB and returns the enriched text.
    pub async fn enrich_task(
        &self,
        task_id: &str,
        task_title: &str,
        task_description: Option<&str>,
    ) -> Result<Option<String>> {
        let current_desc = task_description.unwrap_or("(none)");
        let user_content = format!(
            "Task title: {task_title}\nCurrent description: {current_desc}\n\n\
            Create a clear, structured task description for a Claude Code session (under 250 words). Include:\n\
            1. Clear objective\n\
            2. Acceptance criteria\n\
            3. Likely files or areas of the codebase to focus on\n\
            4. Edge cases to consider\n\
            Keep it actionable and specific. Do not add fictional specifics — only expand on what's given.",
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a technical project manager helping to write clear task descriptions for a software engineer. Be concise and practical.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        info!(task_id = %task_id, task_title = %task_title, model = %self.litellm.model, "Enriching task description");

        match self.litellm.complete(messages).await {
            Ok(result) => {
                let enriched = result.content.trim().to_string();
                info!(
                    task_id = %task_id,
                    input_tokens = result.input_tokens,
                    output_tokens = result.output_tokens,
                    "Task description enriched"
                );
                // Persist the enriched text as instructions (never overwrites user's description)
                use crate::models::UpdateTask;
                if let Err(e) = self.task_repo.update(task_id, UpdateTask {
                    instructions: Some(Some(enriched.clone())),
                    ..Default::default()
                }).await {
                    warn!(task_id = %task_id, error = %e, "Failed to persist enriched task instructions");
                }
                Ok(Some(enriched))
            }
            Err(e) => {
                warn!(task_id = %task_id, error = %e, "Task enrichment failed");
                Err(e)
            }
        }
    }
}
