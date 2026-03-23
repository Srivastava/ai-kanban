use crate::ai::litellm::{ChatMessage, LitellmClient, image_to_data_url};
use crate::db::{AttachmentRepository, CommentRepository, TaskRepository};
use crate::models::CreateComment;
use anyhow::Result;
use tracing::{debug, info, warn};

pub struct ContextManager {
    litellm: LitellmClient,
    comment_repo: CommentRepository,
    task_repo: TaskRepository,
    attachment_repo: AttachmentRepository,
}

impl ContextManager {
    pub fn new(
        litellm: LitellmClient,
        comment_repo: CommentRepository,
        task_repo: TaskRepository,
        attachment_repo: AttachmentRepository,
    ) -> Self {
        Self { litellm, comment_repo, task_repo, attachment_repo }
    }

    /// Fetch image data URLs for all image attachments of a task.
    /// Silently skips any attachment that cannot be read.
    async fn task_image_data_urls(&self, task_id: &str) -> Vec<String> {
        let attachments = match self.attachment_repo.list_for_task(task_id).await {
            Ok(a) => a,
            Err(e) => {
                warn!(task_id = %task_id, error = %e, "Failed to fetch attachments for LiteLLM");
                return vec![];
            }
        };
        let mut urls = Vec::new();
        for att in attachments.iter().filter(|a| a.mime_type.starts_with("image/")) {
            match image_to_data_url(&att.storage_path, &att.mime_type).await {
                Some(url) => {
                    debug!(attachment_id = %att.id, "Encoded image for LiteLLM");
                    urls.push(url);
                }
                None => warn!(attachment_id = %att.id, path = %att.storage_path, "Could not read attachment for LiteLLM"),
            }
        }
        urls
    }

    /// Build a JSON user message that includes text and optional image_url parts.
    fn build_user_message(text: &str, image_data_urls: &[String]) -> serde_json::Value {
        if image_data_urls.is_empty() {
            return serde_json::json!({"role": "user", "content": text});
        }
        let mut parts = vec![serde_json::json!({"type": "text", "text": text})];
        for url in image_data_urls {
            parts.push(serde_json::json!({
                "type": "image_url",
                "image_url": {"url": url}
            }));
        }
        serde_json::json!({"role": "user", "content": parts})
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
        task_stage: &str,
        session_duration_secs: Option<u64>,
        input_tokens: i64,
        output_tokens: i64,
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
            .take(500)
            .map(|s| s.as_str())
            .collect();
        let activity = activity_lines.join("\n");

        let result_section = match result_text {
            Some(r) if !r.is_empty() => {
                let preview = if r.len() > 1200 { &r[..1200] } else { r };
                format!("\n\nFinal output (preview):\n{}", preview)
            }
            _ => String::new(),
        };

        let duration_str = match session_duration_secs {
            Some(d) if d >= 60 => format!("{}m {}s", d / 60, d % 60),
            Some(d) => format!("{}s", d),
            None => "unknown".to_string(),
        };

        let user_content = format!(
            "Task: {task_title} (stage: {task_stage})\n\
             Session stats: {duration_str} · {in_tok} input / {out_tok} output tokens\n\
             Activity ({total} events, showing up to 500):\n{activity}{result_section}\n\n\
             Write a summary using exactly these three sections:\n\
             ## What Changed\n\
             (bullet list of concrete changes — be specific about file names and what was done)\n\n\
             ## Files Modified\n\
             (bullet list: `filename` — one-line description of change)\n\n\
             ## Notes\n\
             (one to three sentences on decisions made, blockers hit, or next steps — omit if nothing notable)",
            task_title = task_title,
            task_stage = task_stage,
            duration_str = duration_str,
            in_tok = input_tokens,
            out_tok = output_tokens,
            total = display_lines.len(),
            activity = activity,
            result_section = result_section,
        );

        let system_msg = serde_json::json!({
            "role": "system",
            "content": "You are a technical project assistant that writes structured session summaries for an AI-assisted development tool. \
Write in clear, specific language. Use exact file names and function names when they appear in the activity log. \
Never use vague phrases like 'various changes' or 'several files'. \
Format output as markdown with the three sections below — fill each one thoroughly. Aim for 200-300 words total."
        });

        let image_urls = self.task_image_data_urls(task_id).await;
        let user_msg = Self::build_user_message(&user_content, &image_urls);

        info!(
            session_id = %session_id,
            task_id = %task_id,
            display_lines = display_lines.len(),
            images = image_urls.len(),
            model = %self.litellm.model,
            "Requesting session summary from LiteLLM"
        );

        match self.litellm.complete_json(vec![system_msg, user_msg]).await {
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

                let perf_line = format!(
                    "⚡ *LiteLLM · {}ms · {:.0} tok/s · {} in / {} out*",
                    result.latency_ms,
                    result.tokens_per_sec,
                    result.input_tokens,
                    result.output_tokens,
                );

                let comment_content = format!(
                    "**Session Summary**\n\n{}{}\n\n---\n{}",
                    result.content.trim(),
                    files_section,
                    perf_line,
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
            Create a structured context summary (under 1500 words) for the next Claude session.\n\
            CRITICAL: If there are any user messages, decisions, or responses in the activity, preserve them verbatim.\n\
            Prioritize recent activity over older activity.\n\
            Include:\n\
            1. What was accomplished\n\
            2. Key decisions and rationale (include exact user responses/choices)\n\
            3. Files modified and why\n\
            4. Current state of the codebase\n\
            5. What remains to be done\n\
            6. Any important constraints or gotchas discovered",
        );

        let system_msg = serde_json::json!({
            "role": "system",
            "content": "You are compressing a Claude Code session into a context handoff for the next session. Be specific, structured, and concise. ALWAYS preserve user messages and decisions verbatim — never paraphrase or drop them. Compress Claude's outputs to save space. Recent activity is more important than older activity."
        });
        let image_urls = self.task_image_data_urls(task_id).await;
        let user_msg = Self::build_user_message(&user_content, &image_urls);

        info!(
            session_id = %session_id,
            task_id = %task_id,
            images = image_urls.len(),
            model = %self.litellm.model,
            "Compressing session context with LiteLLM"
        );

        match self.litellm.complete_json(vec![system_msg, user_msg]).await {
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
            Compress this into a context summary (under 1500 words) for the next Claude session.\n\
            CRITICAL: Preserve ALL user messages verbatim, especially recent ones. If the user responded to a question or made a decision, that response MUST appear in full.\n\
            Prioritize recent messages over older ones — the most recent exchanges are the most important.\n\
            Include:\n\
            - Every user message or decision, quoted directly\n\
            - Key decisions and outcomes\n\
            - Current state\n\
            - What needs to happen next\n\
            - Any blockers or constraints\n\
            Compress Claude's outputs but never compress or lose user messages.",
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are condensing a task conversation history into a context summary for the next Claude session. ALWAYS preserve user messages in full — never paraphrase or drop what the user said. Compress Claude's outputs to save space, but user messages are sacred. Recent messages are more important than older ones.".to_string(),
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
            Create a clear, structured task description for a Claude Code session (under 500 words). Include:\n\
            1. Clear objective\n\
            2. Acceptance criteria\n\
            3. Likely files or areas of the codebase to focus on\n\
            4. Edge cases to consider\n\
            Keep it actionable and specific. Do not add fictional specifics — only expand on what's given.",
        );

        let system_msg = serde_json::json!({
            "role": "system",
            "content": "You are a technical project manager helping to write clear task descriptions for a software engineer. Be concise and practical."
        });

        let image_urls = self.task_image_data_urls(task_id).await;
        let user_msg = Self::build_user_message(&user_content, &image_urls);

        info!(
            task_id = %task_id,
            task_title = %task_title,
            images = image_urls.len(),
            model = %self.litellm.model,
            "Enriching task description"
        );

        match self.litellm.complete_json(vec![system_msg, user_msg]).await {
            Ok(result) => {
                let enriched = result.content.trim().to_string();
                if enriched.is_empty() {
                    warn!(
                        task_id = %task_id,
                        input_tokens = result.input_tokens,
                        output_tokens = result.output_tokens,
                        "LiteLLM returned empty content for task enrichment — skipping (check model configuration)"
                    );
                    return Ok(None);
                }
                info!(
                    task_id = %task_id,
                    input_tokens = result.input_tokens,
                    output_tokens = result.output_tokens,
                    content_len = enriched.len(),
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
                warn!(task_id = %task_id, error = %e, "Task enrichment failed — LiteLLM error, proceeding without enrichment");
                Err(e)
            }
        }
    }
}
