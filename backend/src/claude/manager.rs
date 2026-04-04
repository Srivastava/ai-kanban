use crate::ai::context_manager::ContextManager;
use crate::claude::jsonl_parser::{
    detect_rate_limit_in_stdout, extract_claude_session_id, extract_rate_limit_reset_at,
    extract_result_text, parse_for_display, parse_jsonl_line,
};
use crate::claude::prompts::build_prompt;
use crate::db::{
    AttachmentRepository, CommentRepository, OtelMetricsRepository, SessionMetricsRepository,
    SessionRepository, SettingsRepository, TaskRepository, TokenEventRepository,
};
use crate::models::{CreateTokenEvent, Session, Task, UpdateSession, UpdateTask};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};
use tokio::time::Duration;
use tracing::{debug, info, instrument, warn};

#[derive(Debug, Clone)]
pub enum ClaudeEvent {
    Output {
        session_id: String,
        text: String,
        is_error: bool,
    },
    Heartbeat {
        session_id: String,
        elapsed_secs: u64,
    },
    SessionStatus {
        session_id: String,
        status: String,
    },
    TaskStageChanged {
        task_id: String,
        task_json: serde_json::Value,
    },
    SessionIdAssigned {
        session_id: String,
        claude_session_id: String,
    },
    RateLimited {
        session_id: String,
        task_id: String,
        stage: String,                     // current task stage at time of limit
        claude_session_id: Option<String>, // for --resume on retry
        reset_at: chrono::DateTime<chrono::Utc>,
    },
    /// Emitted at session start — tells frontend which mode Claude is operating in
    StageContextSet {
        session_id: String,
        task_id: String,
        mode: String,
    },
    /// Emitted after the context file (.claude/ai-kanban.md) is written to the project
    ContextFileUpdated {
        session_id: String,
        task_id: String,
    },
    /// Emitted after plan file (.claude/ai-kanban-plan.md) is read back and stored
    PlanCreated {
        session_id: String,
        task_id: String,
        preview: String,
    },
    /// Emitted when LiteLLM task enrichment starts (async — Claude may already be running)
    EnrichmentStarted {
        task_id: String,
    },
    /// Emitted when LiteLLM task enrichment completes successfully
    EnrichmentCompleted {
        task_id: String,
    },
}

struct RunningSession {
    session: Session,
    child: Child,
    task: Task,
}

pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,
    last_session_ended_at: Arc<StdRwLock<Option<Instant>>>,
    output_tx: broadcast::Sender<ClaudeEvent>,
    session_repo: SessionRepository,
    token_event_repo: TokenEventRepository,
    session_metrics_repo: SessionMetricsRepository,
    comment_repo: CommentRepository,
    task_repo: TaskRepository,
    otel_repo: OtelMetricsRepository,
    context_manager: Option<Arc<ContextManager>>,
    settings_repo: Option<SettingsRepository>,
    attachment_repo: AttachmentRepository,
}

/// Minimum peak context size (input + cache_read + cache_creation tokens in any single turn)
/// that triggers LiteLLM context compression at session end.
/// Set to 150K — well below the 200K context window limit, giving enough headroom
/// for the next session to build on the compressed context without immediately hitting limits.
pub const COMPRESSION_TOKEN_THRESHOLD: i64 = 150_000;

impl ClaudeManager {
    pub fn new(
        session_repo: SessionRepository,
        token_event_repo: TokenEventRepository,
        session_metrics_repo: SessionMetricsRepository,
        comment_repo: CommentRepository,
        task_repo: TaskRepository,
        otel_repo: OtelMetricsRepository,
        context_manager: Option<Arc<ContextManager>>,
        settings_repo: Option<SettingsRepository>,
        attachment_repo: AttachmentRepository,
    ) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            last_session_ended_at: Arc::new(StdRwLock::new(None)),
            output_tx,
            session_repo,
            token_event_repo,
            session_metrics_repo,
            comment_repo,
            task_repo,
            otel_repo,
            context_manager,
            settings_repo,
            attachment_repo,
        }
    }

    async fn flag_enabled(&self, key: &str) -> bool {
        match &self.settings_repo {
            Some(repo) => repo.get_flag(key).await.unwrap_or(false),
            None => false,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ClaudeEvent> {
        self.output_tx.subscribe()
    }

    #[instrument(skip(self, task))]
    pub async fn start_session(
        &self,
        mut task: Task,
        stage: &str,
        conversation_context: Option<String>,
        resume_claude_session_id: Option<String>,
    ) -> Result<String> {
        let session = self
            .session_repo
            .create(crate::models::CreateSession {
                task_id: task.id.clone(),
            })
            .await?;

        info!(
            session_id = %session.id,
            task_id = %task.id,
            task_title = %task.title,
            stage = %stage,
            has_context = conversation_context.is_some(),
            "Starting Claude session"
        );

        let claude_bin = std::env::var("CLAUDE_BIN")
            .unwrap_or_else(|_| "/home/utility/.local/bin/claude".to_string());

        // Expand leading ~ to $HOME (the OS does not expand shell tildes)
        let project_path = if task.project_path.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            format!("{}/{}", home, &task.project_path[2..])
        } else {
            task.project_path.clone()
        };

        if !std::path::Path::new(&project_path).exists() {
            return Err(anyhow!("Project path does not exist: {}", project_path));
        }

        // Fetch task attachments
        let attachments = self
            .attachment_repo
            .list_for_task(&task.id)
            .await
            .unwrap_or_default();

        // Fetch comments/discussion for context
        let comments = self
            .comment_repo
            .list_for_task(&task.id)
            .await
            .unwrap_or_default();

        // Write persistent task context to .claude/ai-kanban.md in the project directory.
        // Claude reads the entire .claude/ directory at startup and after compaction, so this
        // survives context resets without needing to re-inject via the prompt.
        write_task_context_file(
            &project_path,
            &task,
            &attachments,
            &comments,
            &self.output_tx,
            &session.id,
        );

        let mut cmd = Command::new(&claude_bin);
        cmd.arg("--print")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions")
            .arg("--output-format")
            .arg("stream-json")
            .current_dir(&project_path)
            // Unset CLAUDECODE so the CLI doesn't refuse to run inside another Claude session
            .env_remove("CLAUDECODE")
            .env("CLAUDE_CODE_ENABLE_TELEMETRY", "1")
            .env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318")
            .env("OTEL_EXPORTER_OTLP_PROTOCOL", "http/json")
            .env("OTEL_METRICS_EXPORTER", "otlp")
            .env("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE", "delta")
            .env("OTEL_LOGS_EXPORTER", "otlp")
            .env("OTEL_EXPORTER_OTLP_LOGS_PROTOCOL", "http/json")
            .env(
                "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
                "http://localhost:4318/v1/logs",
            )
            .env("OTEL_LOG_TOOL_DETAILS", "1")
            .env("OTEL_LOG_USER_PROMPTS", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Copy image attachments into project's .claude/attachments/ and add --image args
        {
            let claude_attachments_dir = format!("{}/.claude/attachments", project_path);
            let _ = tokio::fs::create_dir_all(&claude_attachments_dir).await;
            for att in &attachments {
                let dest = format!("{}/{}-{}", claude_attachments_dir, att.id, att.filename);
                if let Err(e) = tokio::fs::copy(&att.storage_path, &dest).await {
                    warn!(attachment_id = %att.id, error = %e, "Failed to copy attachment to project dir");
                    continue;
                }
                if att.mime_type.starts_with("image/") {
                    cmd.arg("--image").arg(&dest);
                }
            }
        }

        // Task enrichment: on the very first session (no prior history), expand a terse description
        // into structured instructions. Runs in the background so Claude spawns immediately.
        // The enriched instructions are persisted for future sessions and pushed to the frontend via WS.
        if conversation_context.is_none()
            && resume_claude_session_id.is_none()
            && task.session_id.is_none()
        {
            if self.flag_enabled("litellm_task_enrichment").await {
                if let Some(ref cm) = self.context_manager {
                    let cm_bg = cm.clone();
                    let task_id_bg = task.id.clone();
                    let task_title_bg = task.title.clone();
                    let task_desc_bg = task.description.clone();
                    let tx_bg = self.output_tx.clone();
                    let task_repo_bg = self.task_repo.clone();
                    info!(task_id = %task.id, "Spawning async task enrichment");
                    let _ = tx_bg.send(ClaudeEvent::EnrichmentStarted {
                        task_id: task_id_bg.clone(),
                    });
                    tokio::spawn(async move {
                        match cm_bg
                            .enrich_task(&task_id_bg, &task_title_bg, task_desc_bg.as_deref())
                            .await
                        {
                            Ok(Some(_enriched)) => {
                                info!(task_id = %task_id_bg, "Async task enrichment complete");
                                let _ = tx_bg.send(ClaudeEvent::EnrichmentCompleted {
                                    task_id: task_id_bg.clone(),
                                });
                                // Push updated task to frontend
                                if let Ok(updated) = task_repo_bg.find(&task_id_bg).await {
                                    if let Ok(task_json) = serde_json::to_value(&updated) {
                                        let _ = tx_bg.send(ClaudeEvent::TaskStageChanged {
                                            task_id: task_id_bg,
                                            task_json,
                                        });
                                    }
                                }
                            }
                            Ok(None) => {
                                info!(task_id = %task_id_bg, "Async enrichment returned no content");
                            }
                            Err(e) => {
                                warn!(task_id = %task_id_bg, error = %e, "Async task enrichment failed");
                            }
                        }
                    });
                }
            }
        }

        // Pre-session briefing: compress conversation context to reduce token usage
        let conversation_context = if let Some(ref ctx) = conversation_context {
            if self.flag_enabled("litellm_pre_session_briefing").await {
                if let Some(ref cm) = self.context_manager {
                    match cm.generate_briefing(&task.title, ctx).await {
                        Ok(briefing) => {
                            info!(task_id = %task.id, "Pre-session briefing applied");
                            Some(briefing)
                        }
                        Err(_) => Some(ctx.clone()),
                    }
                } else {
                    Some(ctx.clone())
                }
            } else {
                Some(ctx.clone())
            }
        } else {
            None
        };

        // Always build the prompt — it carries the new human-turn message for Claude to act on.
        // When resuming, --resume restores internal conversation state; the prompt is the next request.
        // Use enriched instructions if available, otherwise fall back to user's description.
        // has_plan is true when task.instructions was populated from a prior planning session's
        // .claude/ai-kanban-plan.md file, so the in_progress/review prompt can reference that file.
        let has_plan = task.instructions.is_some();
        let prompt_instructions = task.instructions.as_deref().or(task.description.as_deref());
        let prompt = build_prompt(
            &task.title,
            prompt_instructions,
            stage,
            conversation_context.as_deref(),
            has_plan,
        );

        // Emit StageContextSet before spawning — tells the frontend which mode Claude is in
        let _ = self.output_tx.send(ClaudeEvent::StageContextSet {
            session_id: session.id.clone(),
            task_id: task.id.clone(),
            mode: stage.to_string(),
        });

        if let Some(ref claude_sid) = resume_claude_session_id {
            cmd.arg("--resume").arg(claude_sid);
            info!(
                session_id = %session.id,
                claude_session_id = %claude_sid,
                "Resuming prior Claude session"
            );
        }
        cmd.arg(&prompt);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to spawn Claude (bin={}): {}", claude_bin, e);
                tracing::error!(session_id = %session.id, task_id = %task.id, error = %e, "Claude spawn failed");
                let _ = self
                    .session_repo
                    .update(
                        &session.id,
                        crate::models::UpdateSession {
                            status: Some("failed".to_string()),
                            error_message: Some(err_msg.clone()),
                            ended_at: Some(chrono::Utc::now()),
                            ..Default::default()
                        },
                    )
                    .await;
                return Err(anyhow!(err_msg));
            }
        };

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

        // Shared flag: stdout or stderr reader sets this if it detects a rate-limit message
        let rate_limit_reset: std::sync::Arc<
            std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(None));
        let rate_limit_reset_for_stdout = rate_limit_reset.clone();
        let rate_limit_reset_for_stderr = rate_limit_reset.clone();
        let rate_limit_reset_for_completion = rate_limit_reset.clone();

        let task_id = task.id.clone();
        let task_title = task.title.clone();
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(
                session.id.clone(),
                RunningSession {
                    session: session.clone(),
                    child,
                    task,
                },
            );
        }

        self.session_repo
            .update(
                &session.id,
                crate::models::UpdateSession {
                    status: Some("running".to_string()),
                    ..Default::default()
                },
            )
            .await?;

        // Link session to task and advance to planning — single sequential spawn to avoid race
        let task_repo_init = self.task_repo.clone();
        let task_id_init = task_id.clone();
        let session_id_init = session.id.clone();
        let output_tx_init = self.output_tx.clone();
        tokio::spawn(async move {
            // Step 1: link session_id to task
            if let Err(e) = task_repo_init
                .update(
                    &task_id_init,
                    UpdateTask {
                        session_id: Some(session_id_init),
                        ..Default::default()
                    },
                )
                .await
            {
                warn!(task_id = %task_id_init, error = %e, "Failed to link session_id to task");
            }
            // Step 2: advance to planning
            if let Err(e) = task_repo_init
                .update(
                    &task_id_init,
                    UpdateTask {
                        stage: Some("planning".to_string()),
                        ..Default::default()
                    },
                )
                .await
            {
                warn!(task_id = %task_id_init, error = %e, "Failed to set task stage to planning");
                return;
            }
            // Step 3: fetch task (now has session_id) and broadcast
            match task_repo_init.find(&task_id_init).await {
                Ok(task) => {
                    if let Ok(task_json) = serde_json::to_value(&task) {
                        let _ = output_tx_init.send(ClaudeEvent::TaskStageChanged {
                            task_id: task_id_init.clone(),
                            task_json,
                        });
                    }
                }
                Err(e) => {
                    warn!(task_id = %task_id_init, error = %e, "Failed to fetch task after planning update")
                }
            }
        });

        // Heartbeat: emit every 5s while session is active
        let session_id_hb = session.id.clone();
        let active_sessions_hb = self.active_sessions.clone();
        let output_tx_hb = self.output_tx.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let sessions = active_sessions_hb.read().await;
                if !sessions.contains_key(&session_id_hb) {
                    break;
                }
                drop(sessions);
                let elapsed_secs = start.elapsed().as_secs();
                let _ = output_tx_hb.send(ClaudeEvent::Heartbeat {
                    session_id: session_id_hb.clone(),
                    elapsed_secs,
                });
            }
        });

        // Snapshot project metrics at session start
        let project_metrics = count_project_files(&project_path);
        let metrics_repo = self.session_metrics_repo.clone();
        let session_id_for_metrics = session.id.clone();
        tokio::spawn(async move {
            let _ = metrics_repo
                .upsert(
                    &session_id_for_metrics,
                    project_metrics.0,
                    project_metrics.1,
                )
                .await;
        });

        // Process stdout with JSONL parsing
        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        let token_event_repo = self.token_event_repo.clone();
        let task_id_for_events = task_id.clone();
        let task_repo_for_stage = self.task_repo.clone();
        let output_tx_for_stage = self.output_tx.clone();
        let session_repo_for_stdout = self.session_repo.clone();
        let otel_repo_for_stdout = self.otel_repo.clone();
        let task_id_for_otel = task_id.clone();
        let stdout_handle = tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            let mut sequence_no: i64 = 0;
            let mut result_text: Option<String> = None;
            let mut display_lines: Vec<String> = Vec::new();
            let mut peak_input_tokens: i64 = 0;
            let mut total_output_tokens: i64 = 0;
            let mut first_tool_seen = false;
            let mut claude_session_id_captured = false;
            // Deduplication buffer: keep only the LAST event per message_id.
            // Claude emits 2 assistant events per API call (streaming-start with output=0,
            // then final with output=N). Both share the same message_id. By keeping only
            // the last, we get correct output counts without double-counting input/cache.
            let mut pending_events: HashMap<String, (CreateTokenEvent, i64)> = HashMap::new();
            for line in reader.lines() {
                if let Ok(text) = line {
                    debug!(session_id = %session_id, "stdout: {}", text);

                    // Detect rate-limit signals in stdout (Claude emits usage-limit errors
                    // to stdout as JSON result events, not to stderr)
                    if let Some(reset_at) = detect_rate_limit_in_stdout(&text) {
                        let mut guard = rate_limit_reset_for_stdout.lock().unwrap();
                        if guard.is_none() {
                            warn!(session_id = %session_id, reset_at = %reset_at, "Rate limit detected in stdout");
                            *guard = Some(reset_at);
                        }
                    }

                    // Capture Claude's internal session_id from the init event (first line)
                    if !claude_session_id_captured {
                        if let Some(claude_sid) = extract_claude_session_id(&text) {
                            claude_session_id_captured = true;
                            let rt = tokio::runtime::Handle::current();
                            let session_repo_c = session_repo_for_stdout.clone();
                            let otel_repo_c = otel_repo_for_stdout.clone();
                            let session_id_c = session_id.clone();
                            let task_id_c = task_id_for_otel.clone();
                            let claude_sid_c = claude_sid.clone();
                            let output_tx_c = output_tx.clone();
                            rt.spawn(async move {
                                let _ = session_repo_c
                                    .update(
                                        &session_id_c,
                                        crate::models::UpdateSession {
                                            claude_session_id: Some(claude_sid_c.clone()),
                                            ..Default::default()
                                        },
                                    )
                                    .await;
                                // Backfill any OTel metrics that arrived before the session
                                // was assigned its claude_session_id (timing race at startup)
                                match otel_repo_c
                                    .correlate(&claude_sid_c, &session_id_c, &task_id_c)
                                    .await
                                {
                                    Ok(()) => tracing::debug!(
                                        session_id = %session_id_c,
                                        claude_session_id = %claude_sid_c,
                                        task_id = %task_id_c,
                                        "OTel metrics correlated"
                                    ),
                                    Err(e) => tracing::warn!(
                                        session_id = %session_id_c,
                                        error = %e,
                                        "Failed to correlate OTel metrics"
                                    ),
                                }
                                let _ = output_tx_c.send(ClaudeEvent::SessionIdAssigned {
                                    session_id: session_id_c,
                                    claude_session_id: claude_sid_c,
                                });
                            });
                        }
                    }

                    // Try to parse as JSONL and extract token events
                    if let Some(parsed) = parse_jsonl_line(&text) {
                        // Track peak context size for compression decision.
                        // Total context = input (non-cached) + cache_read + cache_creation.
                        // We must use the full context size, not just input_tokens, because
                        // in long sessions almost all context is cache-read (e.g. 695K tokens)
                        // while non-cached input is tiny (< 100 tokens).
                        let total_context = parsed.input_tokens
                            + parsed.cache_read_tokens
                            + parsed.cache_creation_tokens;
                        if total_context > peak_input_tokens {
                            peak_input_tokens = total_context;
                        }
                        // Accumulate output tokens for post-session summarization
                        total_output_tokens += parsed.output_tokens;

                        let event = CreateTokenEvent {
                            session_id: session_id.clone(),
                            task_id: task_id_for_events.clone(),
                            event_type: parsed.event_type,
                            tool_name: parsed.tool_name,
                            file_ext: parsed.file_ext,
                            input_tokens: parsed.input_tokens,
                            output_tokens: parsed.output_tokens,
                            cache_read_tokens: parsed.cache_read_tokens,
                            cache_creation_tokens: parsed.cache_creation_tokens,
                            model: parsed.model,
                            sequence_no: Some(sequence_no),
                        };
                        sequence_no += 1;

                        match parsed.message_id {
                            Some(msg_id) => {
                                // Buffer: keep only the last event for this message_id.
                                // Claude emits streaming-start (output=0) then final (output=N)
                                // — both have the same message_id, same input/cache counts.
                                pending_events.insert(msg_id, (event, sequence_no));
                            }
                            None => {
                                // result event — cumulative session totals, would double-count.
                                // Flush all buffered assistant events instead.
                                let rt = tokio::runtime::Handle::current();
                                for (pending_event, _seq) in pending_events.drain().map(|(_, v)| v)
                                {
                                    let repo = token_event_repo.clone();
                                    rt.spawn(async move {
                                        let _ = repo.create(pending_event).await;
                                    });
                                }
                            }
                        }
                    }

                    if let Some(r) = extract_result_text(&text) {
                        result_text = Some(r);
                    }

                    // Parse for display and emit human-readable output
                    let (display_text, has_tool) = parse_for_display(&text);

                    // First tool use → advance task to in_progress
                    if has_tool && !first_tool_seen {
                        first_tool_seen = true;
                        let rt = tokio::runtime::Handle::current();
                        let task_repo_tool = task_repo_for_stage.clone();
                        let task_id_tool = task_id_for_events.clone();
                        let output_tx_tool = output_tx_for_stage.clone();
                        rt.spawn(async move {
                            if let Err(e) = task_repo_tool.update(&task_id_tool, UpdateTask {
                                stage: Some("in_progress".to_string()),
                                ..Default::default()
                            }).await {
                                warn!(task_id = %task_id_tool, error = %e, "Failed to set task stage to in_progress");
                                return;
                            }
                            match task_repo_tool.find(&task_id_tool).await {
                                Ok(task) => {
                                    if let Ok(task_json) = serde_json::to_value(&task) {
                                        let _ = output_tx_tool.send(ClaudeEvent::TaskStageChanged {
                                            task_id: task_id_tool.clone(),
                                            task_json,
                                        });
                                    }
                                }
                                Err(e) => warn!(task_id = %task_id_tool, error = %e, "Failed to fetch task after in_progress update"),
                            }
                        });
                    }

                    // Emit display line (only if there's something to show)
                    if let Some(display) = display_text {
                        // Collect up to 500 lines for post-session summarization
                        if display_lines.len() < 500 {
                            display_lines.push(display.clone());
                        }
                        let _ = output_tx.send(ClaudeEvent::Output {
                            session_id: session_id.clone(),
                            text: display,
                            is_error: false,
                        });
                    }
                }
            }
            // Flush any remaining buffered events (session ended without a result event,
            // or last API call's final event hasn't been superseded).
            if !pending_events.is_empty() {
                let rt = tokio::runtime::Handle::current();
                for (pending_event, _seq) in pending_events.drain().map(|(_, v)| v) {
                    let repo = token_event_repo.clone();
                    rt.spawn(async move {
                        let _ = repo.create(pending_event).await;
                    });
                }
            }
            (result_text, display_lines, peak_input_tokens, total_output_tokens)
        });

        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        let stderr_handle = tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(text) = line {
                    warn!(session_id = %session_id, "stderr: {}", text);
                    if let Some(reset_at) = extract_rate_limit_reset_at(&text) {
                        let mut guard = rate_limit_reset_for_stderr.lock().unwrap();
                        *guard = Some(reset_at);
                    }
                    let _ = output_tx.send(ClaudeEvent::Output {
                        session_id: session_id.clone(),
                        text,
                        is_error: true,
                    });
                }
            }
        });

        // Completion task: once stdout+stderr close, wait on child, update session status
        let session_id_for_completion = session.id.clone();
        let active_sessions_for_completion = self.active_sessions.clone();
        let last_ended_for_completion = self.last_session_ended_at.clone();
        let session_repo_for_completion = self.session_repo.clone();
        let comment_repo_for_completion = self.comment_repo.clone();
        let output_tx_for_completion = self.output_tx.clone();
        let task_repo_for_completion = self.task_repo.clone();
        let context_manager_for_completion = self.context_manager.clone();
        let settings_repo_for_completion = self.settings_repo.clone();
        let task_title_for_completion = task_title;
        let stage_for_completion = stage.to_string();
        let project_path_for_completion = project_path.clone();
        tokio::spawn(async move {
            // Wait for both I/O reader threads to finish (they exit when streams close)
            let (result_text, display_lines, peak_input_tokens, total_output_tokens) =
                match stdout_handle.await {
                    Ok(quad) => quad,
                    Err(_) => (None, Vec::new(), 0i64, 0i64),
                };
            let _ = stderr_handle.await;

            // Take the child out of active_sessions
            let child_opt = {
                let mut sessions = active_sessions_for_completion.write().await;
                let child = sessions
                    .remove(&session_id_for_completion)
                    .map(|rs| rs.child);
                if let Ok(mut ts) = last_ended_for_completion.write() {
                    *ts = Some(Instant::now());
                }
                child
            };

            // Wait on child to reap the zombie and get exit status
            let exit_ok = if let Some(mut child) = child_opt {
                match tokio::task::spawn_blocking(move || child.wait()).await {
                    Ok(Ok(status)) => {
                        info!(
                            session_id = %session_id_for_completion,
                            exit_code = ?status.code(),
                            success = status.success(),
                            "Claude process exited"
                        );
                        status.success()
                    }
                    _ => false,
                }
            } else {
                // Session was already removed (e.g. manually stopped)
                return;
            };

            // Check if this was a rate-limit exit
            let rate_limit_reset_at = rate_limit_reset_for_completion.lock().unwrap().take();

            let final_status = if exit_ok {
                "completed"
            } else if rate_limit_reset_at.is_some() {
                "stopped" // not "failed" — will be auto-retried
            } else {
                "failed"
            };

            info!(
                session_id = %session_id_for_completion,
                status = %final_status,
                "Session finished"
            );
            let _ = session_repo_for_completion
                .update(
                    &session_id_for_completion,
                    UpdateSession {
                        status: Some(final_status.to_string()),
                        ended_at: Some(chrono::Utc::now()),
                        error_message: rate_limit_reset_at
                            .as_ref()
                            .map(|dt| format!("rate_limited:{}", dt.to_rfc3339())),
                        ..Default::default()
                    },
                )
                .await;

            // Emit session status via WS
            let _ = output_tx_for_completion.send(ClaudeEvent::SessionStatus {
                session_id: session_id_for_completion.clone(),
                status: final_status.to_string(),
            });

            // If rate-limited: emit RateLimited event.
            // If Claude was actually interrupted (exit_ok=false), skip normal completion handling.
            // If Claude finished cleanly despite the rate-limit signal (exit_ok=true), fall through
            // so comments and stage advancement still happen.
            if let Some(reset_at) = rate_limit_reset_at {
                if let Ok(session) = session_repo_for_completion
                    .find(&session_id_for_completion)
                    .await
                {
                    if let Ok(task) = task_repo_for_completion.find(&session.task_id).await {
                        let _ = output_tx_for_completion.send(ClaudeEvent::RateLimited {
                            session_id: session_id_for_completion.clone(),
                            task_id: session.task_id.clone(),
                            stage: task.stage.clone(),
                            claude_session_id: session.claude_session_id,
                            reset_at,
                        });
                    }
                }
                if !exit_ok {
                    return; // Claude was interrupted — skip comment posting and stage advancement
                }
            }

            // On success: read back plan file if Claude wrote one during the planning phase.
            // Claude is instructed to write its plan to .claude/ai-kanban-plan.md.
            // We store the plan as task.instructions so future sessions can reference it.
            if exit_ok {
                let plan_path =
                    format!("{}/.claude/ai-kanban-plan.md", project_path_for_completion);
                if let Ok(plan_content) = std::fs::read_to_string(&plan_path) {
                    let plan_content = plan_content.trim().to_string();
                    if !plan_content.is_empty() {
                        if let Ok(session) = session_repo_for_completion
                            .find(&session_id_for_completion)
                            .await
                        {
                            match task_repo_for_completion
                                .update(
                                    &session.task_id,
                                    UpdateTask {
                                        instructions: Some(Some(plan_content.clone())),
                                        ..Default::default()
                                    },
                                )
                                .await
                            {
                                Ok(_) => {
                                    info!(
                                        task_id = %session.task_id,
                                        "Stored Claude's plan from .claude/ai-kanban-plan.md as task instructions"
                                    );
                                    // Emit PlanCreated event with first 200 chars as preview
                                    let preview: String = plan_content.chars().take(200).collect();
                                    let _ =
                                        output_tx_for_completion.send(ClaudeEvent::PlanCreated {
                                            session_id: session_id_for_completion.clone(),
                                            task_id: session.task_id.clone(),
                                            preview,
                                        });
                                    // Notify frontend so Instructions section refreshes
                                    if let Ok(updated_task) =
                                        task_repo_for_completion.find(&session.task_id).await
                                    {
                                        if let Ok(task_json) = serde_json::to_value(&updated_task) {
                                            let _ = output_tx_for_completion.send(
                                                ClaudeEvent::TaskStageChanged {
                                                    task_id: session.task_id.clone(),
                                                    task_json,
                                                },
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(task_id = %session.task_id, error = %e, "Failed to store plan as instructions")
                                }
                            }
                        }
                    }
                }
            }

            // On success: advance task to review stage
            if exit_ok {
                match session_repo_for_completion
                    .find(&session_id_for_completion)
                    .await
                {
                    Ok(session) => {
                        if let Err(e) = task_repo_for_completion
                            .update(
                                &session.task_id,
                                UpdateTask {
                                    stage: Some("review".to_string()),
                                    ..Default::default()
                                },
                            )
                            .await
                        {
                            warn!(task_id = %session.task_id, error = %e, "Failed to set task stage to review");
                        } else {
                            match task_repo_for_completion.find(&session.task_id).await {
                                Ok(task) => {
                                    if let Ok(task_json) = serde_json::to_value(&task) {
                                        let _ = output_tx_for_completion.send(
                                            ClaudeEvent::TaskStageChanged {
                                                task_id: session.task_id.clone(),
                                                task_json,
                                            },
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!(task_id = %session.task_id, error = %e, "Failed to fetch task after review update")
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(session_id = %session_id_for_completion, error = %e, "Failed to fetch session for task review update")
                    }
                }
            }

            // Post Claude's response as a comment
            if exit_ok {
                if let Some(ref text) = result_text {
                    if !text.is_empty() {
                        // We need the task_id — fetch it from the session record
                        if let Ok(session) = session_repo_for_completion
                            .find(&session_id_for_completion)
                            .await
                        {
                            use crate::models::CreateComment;
                            let content_len = text.len();
                            let preview = text.chars().take(120).collect::<String>();
                            let preview = if content_len > 120 {
                                format!("{}…", preview)
                            } else {
                                preview
                            };
                            match comment_repo_for_completion
                                .create(
                                    &session.task_id,
                                    "claude",
                                    CreateComment {
                                        content: text.clone(),
                                        parent_id: None,
                                    },
                                )
                                .await
                            {
                                Ok(comment) => info!(
                                    session_id = %session_id_for_completion,
                                    task_id = %session.task_id,
                                    comment_id = %comment.id,
                                    content_len = content_len,
                                    preview = %preview,
                                    "Posted Claude result as comment"
                                ),
                                Err(e) => warn!(
                                    session_id = %session_id_for_completion,
                                    task_id = %session.task_id,
                                    error = %e,
                                    "Failed to post Claude result as comment"
                                ),
                            }
                        }
                    }
                } else {
                    info!(session_id = %session_id_for_completion, "Claude session completed with no result text");
                }

                // Post-session: generate LiteLLM summary of what Claude did
                if let Some(ref ctx_mgr) = context_manager_for_completion {
                    if let Ok(session) = session_repo_for_completion
                        .find(&session_id_for_completion)
                        .await
                    {
                        // Session summary (always runs if flag enabled)
                        let summary_enabled =
                            settings_repo_for_completion.as_ref().map(|r| async move {
                                r.get_flag("litellm_session_summary").await.unwrap_or(true)
                            });
                        let do_summary = match summary_enabled {
                            Some(f) => f.await,
                            None => true, // default on
                        };
                        if do_summary {
                            let session_duration_secs = session.ended_at.map(|ended| {
                                (ended - session.started_at).num_seconds().max(0) as u64
                            });
                            let _ = ctx_mgr
                                .summarize_session(
                                    &session_id_for_completion,
                                    &session.task_id,
                                    &task_title_for_completion,
                                    &stage_for_completion,
                                    session_duration_secs,
                                    peak_input_tokens,
                                    total_output_tokens,
                                    &display_lines,
                                    result_text.as_deref(),
                                )
                                .await;
                        }

                        // Context compression when peak context size approaches the threshold.
                        // "peak_input_tokens" = max(input + cache_read + cache_creation) across
                        // all JSONL turns — it reflects the largest context window used this session.
                        let pct_of_threshold = (peak_input_tokens as f64
                            / COMPRESSION_TOKEN_THRESHOLD as f64
                            * 100.0)
                            .round() as i64;
                        if peak_input_tokens >= COMPRESSION_TOKEN_THRESHOLD {
                            if let Some(repo) = settings_repo_for_completion.as_ref() {
                                let flag_enabled = repo
                                    .get_flag("litellm_context_compression")
                                    .await
                                    .unwrap_or(false);
                                if flag_enabled {
                                    info!(
                                        session_id = %session_id_for_completion,
                                        task_id = %session.task_id,
                                        peak_input_tokens = peak_input_tokens,
                                        threshold = COMPRESSION_TOKEN_THRESHOLD,
                                        pct_of_threshold = pct_of_threshold,
                                        "Token threshold reached — compressing context"
                                    );
                                    if let Err(e) = ctx_mgr
                                        .compress_context(
                                            &session_id_for_completion,
                                            &session.task_id,
                                            &task_title_for_completion,
                                            &display_lines,
                                            result_text.as_deref(),
                                        )
                                        .await
                                    {
                                        warn!(
                                            session_id = %session_id_for_completion,
                                            task_id = %session.task_id,
                                            error = %e,
                                            "Context compression failed — next session will run without compressed context"
                                        );
                                    } else {
                                        // Notify frontend so context section refreshes
                                        if let Ok(updated) =
                                            task_repo_for_completion.find(&session.task_id).await
                                        {
                                            if let Ok(task_json) = serde_json::to_value(&updated) {
                                                let _ = output_tx_for_completion.send(
                                                    ClaudeEvent::TaskStageChanged {
                                                        task_id: session.task_id.clone(),
                                                        task_json,
                                                    },
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    warn!(
                                        session_id = %session_id_for_completion,
                                        task_id = %session.task_id,
                                        peak_input_tokens = peak_input_tokens,
                                        threshold = COMPRESSION_TOKEN_THRESHOLD,
                                        pct_of_threshold = pct_of_threshold,
                                        "Token threshold reached but litellm_context_compression flag is disabled — context will NOT be compressed"
                                    );
                                }
                            }
                        } else {
                            // Log at debug for low-token sessions, info for sessions
                            // within 20% of the threshold so we can spot trends.
                            if pct_of_threshold >= 80 {
                                info!(
                                    session_id = %session_id_for_completion,
                                    task_id = %session.task_id,
                                    peak_input_tokens = peak_input_tokens,
                                    threshold = COMPRESSION_TOKEN_THRESHOLD,
                                    pct_of_threshold = pct_of_threshold,
                                    "Session used {}% of compression threshold — approaching limit",
                                    pct_of_threshold
                                );
                            } else {
                                tracing::debug!(
                                    session_id = %session_id_for_completion,
                                    peak_input_tokens = peak_input_tokens,
                                    pct_of_threshold = pct_of_threshold,
                                    "Session ended below compression threshold"
                                );
                            }
                        }
                    }
                }
            }
        });

        Ok(session.id)
    }

    #[instrument(skip(self))]
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        // Remove from active sessions and kill child (under write lock)
        let removed = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(session_id).map(|mut rs| {
                info!(
                    session_id = %session_id,
                    task_id = %rs.task.id,
                    task_title = %rs.task.title,
                    "Stopping session"
                );
                let _ = rs.child.kill();
                rs
            })
        }; // write lock dropped here

        if removed.is_some() {
            // Update DB (no lock held)
            self.session_repo
                .update(
                    session_id,
                    crate::models::UpdateSession {
                        status: Some("stopped".to_string()),
                        ended_at: Some(chrono::Utc::now()),
                        ..Default::default()
                    },
                )
                .await?;

            // Notify WS clients that the session has stopped
            let _ = self.output_tx.send(ClaudeEvent::SessionStatus {
                session_id: session_id.to_string(),
                status: "stopped".to_string(),
            });
        }
        Ok(())
    }

    pub async fn active_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    /// Returns true if a session is currently running OR ended within the last 30 minutes.
    /// Note: resets to false on server restart (Instant is not persisted).
    pub async fn recently_active(&self) -> bool {
        // active_sessions uses tokio RwLock → .read().await
        if self.active_sessions.read().await.len() > 0 {
            return true;
        }
        // last_session_ended_at uses std RwLock → .read() returns Result (not a future)
        if let Ok(ts) = self.last_session_ended_at.read() {
            if let Some(ended) = *ts {
                return ended.elapsed() < std::time::Duration::from_secs(30 * 60);
            }
        }
        false
    }

    pub async fn is_active(&self, session_id: &str) -> bool {
        self.active_sessions.read().await.contains_key(session_id)
    }

    pub async fn get_active_session_for_task(&self, task_id: &str) -> Option<String> {
        let sessions = self.active_sessions.read().await;
        sessions
            .iter()
            .find(|(_, rs)| rs.task.id == task_id)
            .map(|(session_id, _)| session_id.clone())
    }
}

/// Write (or overwrite) `.claude/ai-kanban.md` in the project directory.
///
/// Claude Code reads the entire `.claude/` directory at startup and re-reads it
/// after context compaction, so this file survives within-session compaction without
/// any extra prompt injection. Each session start refreshes it with the latest state.
///
/// Emits `ClaudeEvent::ContextFileUpdated` on successful write.
pub fn write_task_context_file(
    project_path: &str,
    task: &Task,
    attachments: &[crate::models::TaskAttachment],
    comments: &[crate::models::CommentWithReplies],
    tx: &broadcast::Sender<ClaudeEvent>,
    session_id: &str,
) {
    let claude_dir = std::path::Path::new(project_path).join(".claude");
    if let Err(e) = std::fs::create_dir_all(&claude_dir) {
        warn!(
            project_path = %project_path,
            task_id = %task.id,
            session_id = %session_id,
            error = %e,
            "Could not create .claude dir for ai-kanban context file"
        );
        return;
    }

    let mut lines = vec![
        "<!-- This file is managed by AI Kanban. Do not edit manually. -->".to_string(),
        format!("# Task: {}", task.title),
    ];

    if let Some(ref desc) = task.description {
        if !desc.trim().is_empty() {
            lines.push(String::new());
            lines.push("## Description".to_string());
            lines.push(desc.clone());
        }
    }

    if let Some(ref instructions) = task.instructions {
        if !instructions.trim().is_empty() {
            lines.push(String::new());
            lines.push("## Implementation Plan".to_string());
            lines.push(instructions.clone());
        }
    }

    if let Some(ref ctx) = task.compressed_context {
        if !ctx.trim().is_empty() {
            lines.push(String::new());
            lines.push("## Prior Session Context (compressed)".to_string());
            lines.push(ctx.clone());
        }
    }

    // Include comments and replies so Claude has full conversation context
    let user_comments: Vec<_> = comments
        .iter()
        .filter(|c| c.comment.author != "litellm")
        .collect();
    if !user_comments.is_empty() {
        lines.push(String::new());
        lines.push("## Discussion".to_string());
        lines.push(
            "(Previous comments and replies — most recent context for this task)".to_string(),
        );
        for thread in &user_comments {
            let c = &thread.comment;
            let author_label = if c.author == "claude" {
                "Claude"
            } else {
                "User"
            };
            let ts = c.created_at.format("%Y-%m-%d %H:%M UTC").to_string();
            lines.push(String::new());
            lines.push(format!("**{}** [{}]:", author_label, ts));
            lines.push(c.content.clone());
            for reply in &thread.replies {
                let reply_author = if reply.author == "claude" {
                    "Claude"
                } else {
                    "User"
                };
                let reply_ts = reply.created_at.format("%Y-%m-%d %H:%M UTC").to_string();
                lines.push(String::new());
                lines.push(format!("  > **{}** [{}]:", reply_author, reply_ts));
                for line in reply.content.lines() {
                    lines.push(format!("  > {}", line));
                }
            }
        }
    }

    if !attachments.is_empty() {
        lines.push(String::new());
        lines.push("## Attached Files".to_string());
        for att in attachments {
            lines.push(format!(
                "- `.claude/attachments/{}-{}` ({})",
                att.id, att.filename, att.mime_type
            ));
        }
        lines.push(String::new());
        lines.push(
            "Please review the attached files as they are relevant to this task.".to_string(),
        );
    }

    lines.push(String::new());
    lines.push(format!(
        "<!-- Task ID: {} | Stage: {} -->",
        task.id, task.stage
    ));

    let content = lines.join("\n");
    let file_path = claude_dir.join("ai-kanban.md");
    if let Err(e) = std::fs::write(&file_path, content) {
        warn!(
            path = ?file_path,
            task_id = %task.id,
            session_id = %session_id,
            error = %e,
            "Failed to write .claude/ai-kanban.md"
        );
    } else {
        info!(task_id = %task.id, path = ?file_path, "Wrote task context to .claude/ai-kanban.md");
        let _ = tx.send(ClaudeEvent::ContextFileUpdated {
            session_id: session_id.to_string(),
            task_id: task.id.clone(),
        });
    }
}

fn count_project_files(project_path: &str) -> (i64, i64) {
    use std::fs;
    let mut file_count: i64 = 0;
    let mut loc: i64 = 0;

    fn visit_dir(path: &std::path::Path, file_count: &mut i64, loc: &mut i64) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
            }
            if p.is_dir() {
                visit_dir(&p, file_count, loc);
            } else if p.is_file() {
                *file_count += 1;
                if let Ok(content) = fs::read_to_string(&p) {
                    *loc += content.lines().count() as i64;
                }
            }
        }
    }

    visit_dir(
        std::path::Path::new(project_path),
        &mut file_count,
        &mut loc,
    );
    (file_count, loc)
}
