use crate::claude::jsonl_parser::{extract_claude_session_id, extract_rate_limit_reset_at, extract_result_text, parse_for_display, parse_jsonl_line};
use crate::claude::prompts::build_prompt;
use crate::db::{CommentRepository, SessionMetricsRepository, SessionRepository, TaskRepository, TokenEventRepository};
use crate::models::{CreateTokenEvent, Session, Task, UpdateSession, UpdateTask};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
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
        stage: String,                        // current task stage at time of limit
        claude_session_id: Option<String>,    // for --resume on retry
        reset_at: chrono::DateTime<chrono::Utc>,
    },
}

struct RunningSession {
    session: Session,
    child: Child,
    task: Task,
}

pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,
    output_tx: broadcast::Sender<ClaudeEvent>,
    session_repo: SessionRepository,
    token_event_repo: TokenEventRepository,
    session_metrics_repo: SessionMetricsRepository,
    comment_repo: CommentRepository,
    task_repo: TaskRepository,
}

impl ClaudeManager {
    pub fn new(
        session_repo: SessionRepository,
        token_event_repo: TokenEventRepository,
        session_metrics_repo: SessionMetricsRepository,
        comment_repo: CommentRepository,
        task_repo: TaskRepository,
    ) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            session_repo,
            token_event_repo,
            session_metrics_repo,
            comment_repo,
            task_repo,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ClaudeEvent> {
        self.output_tx.subscribe()
    }

    #[instrument(skip(self, task))]
    pub async fn start_session(&self, task: Task, stage: &str, conversation_context: Option<String>, resume_claude_session_id: Option<String>) -> Result<String> {
        let session = self.session_repo.create(crate::models::CreateSession {
            task_id: task.id.clone(),
        }).await?;

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

        let mut cmd = Command::new(&claude_bin);
        cmd.arg("--print")
           .arg("--verbose")
           .arg("--dangerously-skip-permissions")
           .arg("--output-format").arg("stream-json")
           .current_dir(&project_path)
           // Unset CLAUDECODE so the CLI doesn't refuse to run inside another Claude session
           .env_remove("CLAUDECODE")
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        if let Some(ref claude_sid) = resume_claude_session_id {
            // True resume: Claude already has full conversation history internally
            cmd.arg("--resume").arg(claude_sid);
            info!(
                session_id = %session.id,
                claude_session_id = %claude_sid,
                "Resuming prior Claude session"
            );
        } else {
            // New session: inject prompt (with optional conversation history)
            let prompt = build_prompt(&task.title, task.description.as_deref(), stage, conversation_context.as_deref());
            cmd.arg(&prompt);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn Claude (bin={}): {}", claude_bin, e))?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

        // Shared flag: stderr reader sets this if it detects a rate-limit message
        let rate_limit_reset: std::sync::Arc<std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let rate_limit_reset_for_stderr = rate_limit_reset.clone();
        let rate_limit_reset_for_completion = rate_limit_reset.clone();

        let task_id = task.id.clone();
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session.id.clone(), RunningSession {
                session: session.clone(),
                child,
                task,
            });
        }

        self.session_repo.update(&session.id, crate::models::UpdateSession {
            status: Some("running".to_string()),
            ..Default::default()
        }).await?;

        // Link session to task and advance to planning — single sequential spawn to avoid race
        let task_repo_init = self.task_repo.clone();
        let task_id_init = task_id.clone();
        let session_id_init = session.id.clone();
        let output_tx_init = self.output_tx.clone();
        tokio::spawn(async move {
            // Step 1: link session_id to task
            if let Err(e) = task_repo_init.update(&task_id_init, UpdateTask {
                session_id: Some(session_id_init),
                ..Default::default()
            }).await {
                warn!(task_id = %task_id_init, error = %e, "Failed to link session_id to task");
            }
            // Step 2: advance to planning
            if let Err(e) = task_repo_init.update(&task_id_init, UpdateTask {
                stage: Some("planning".to_string()),
                ..Default::default()
            }).await {
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
                Err(e) => warn!(task_id = %task_id_init, error = %e, "Failed to fetch task after planning update"),
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
            let _ = metrics_repo.upsert(&session_id_for_metrics, project_metrics.0, project_metrics.1).await;
        });

        // Process stdout with JSONL parsing
        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        let token_event_repo = self.token_event_repo.clone();
        let task_id_for_events = task_id.clone();
        let task_repo_for_stage = self.task_repo.clone();
        let output_tx_for_stage = self.output_tx.clone();
        let session_repo_for_stdout = self.session_repo.clone();
        let stdout_handle = tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            let mut sequence_no: i64 = 0;
            let mut result_text: Option<String> = None;
            let mut first_tool_seen = false;
            let mut claude_session_id_captured = false;
            for line in reader.lines() {
                if let Ok(text) = line {
                    debug!(session_id = %session_id, "stdout: {}", text);

                    // Capture Claude's internal session_id from the init event (first line)
                    if !claude_session_id_captured {
                        if let Some(claude_sid) = extract_claude_session_id(&text) {
                            claude_session_id_captured = true;
                            let rt = tokio::runtime::Handle::current();
                            let session_repo_c = session_repo_for_stdout.clone();
                            let session_id_c = session_id.clone();
                            let claude_sid_c = claude_sid.clone();
                            let output_tx_c = output_tx.clone();
                            rt.spawn(async move {
                                let _ = session_repo_c.update(&session_id_c, crate::models::UpdateSession {
                                    claude_session_id: Some(claude_sid_c.clone()),
                                    ..Default::default()
                                }).await;
                                let _ = output_tx_c.send(ClaudeEvent::SessionIdAssigned {
                                    session_id: session_id_c,
                                    claude_session_id: claude_sid_c,
                                });
                            });
                        }
                    }

                    // Try to parse as JSONL and extract token events
                    if let Some(parsed) = parse_jsonl_line(&text) {
                        let rt = tokio::runtime::Handle::current();
                        let event = CreateTokenEvent {
                            session_id: session_id.clone(),
                            task_id: task_id_for_events.clone(),
                            event_type: parsed.event_type,
                            tool_name: parsed.tool_name,
                            file_ext: parsed.file_ext,
                            input_tokens: parsed.input_tokens,
                            output_tokens: parsed.output_tokens,
                            model: parsed.model,
                            sequence_no: Some(sequence_no),
                        };
                        sequence_no += 1;

                        let repo = token_event_repo.clone();
                        rt.spawn(async move {
                            let _ = repo.create(event).await;
                        });
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
                        let _ = output_tx.send(ClaudeEvent::Output {
                            session_id: session_id.clone(),
                            text: display,
                            is_error: false,
                        });
                    }
                }
            }
            result_text
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
        let session_repo_for_completion = self.session_repo.clone();
        let comment_repo_for_completion = self.comment_repo.clone();
        let output_tx_for_completion = self.output_tx.clone();
        let task_repo_for_completion = self.task_repo.clone();
        tokio::spawn(async move {
            // Wait for both I/O reader threads to finish (they exit when streams close)
            let result_text = match stdout_handle.await {
                Ok(text) => text,   // stdout_handle now returns Option<String>
                Err(_) => None,
            };
            let _ = stderr_handle.await;

            // Take the child out of active_sessions
            let child_opt = {
                let mut sessions = active_sessions_for_completion.write().await;
                sessions.remove(&session_id_for_completion).map(|rs| rs.child)
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
                "stopped"   // not "failed" — will be auto-retried
            } else {
                "failed"
            };

            info!(
                session_id = %session_id_for_completion,
                status = %final_status,
                "Session finished"
            );
            let _ = session_repo_for_completion.update(&session_id_for_completion, UpdateSession {
                status: Some(final_status.to_string()),
                ended_at: Some(chrono::Utc::now()),
                error_message: rate_limit_reset_at.as_ref().map(|dt| {
                    format!("rate_limited:{}", dt.to_rfc3339())
                }),
                ..Default::default()
            }).await;

            // Emit session status via WS
            let _ = output_tx_for_completion.send(ClaudeEvent::SessionStatus {
                session_id: session_id_for_completion.clone(),
                status: final_status.to_string(),
            });

            // If rate-limited: emit RateLimited event and skip normal completion handling
            if let Some(reset_at) = rate_limit_reset_at {
                if let Ok(session) = session_repo_for_completion.find(&session_id_for_completion).await {
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
                return; // skip normal success/failure handling (review stage, comment posting)
            }

            // On success: advance task to review stage
            if exit_ok {
                match session_repo_for_completion.find(&session_id_for_completion).await {
                    Ok(session) => {
                        if let Err(e) = task_repo_for_completion.update(&session.task_id, UpdateTask {
                            stage: Some("review".to_string()),
                            ..Default::default()
                        }).await {
                            warn!(task_id = %session.task_id, error = %e, "Failed to set task stage to review");
                        } else {
                            match task_repo_for_completion.find(&session.task_id).await {
                                Ok(task) => {
                                    if let Ok(task_json) = serde_json::to_value(&task) {
                                        let _ = output_tx_for_completion.send(ClaudeEvent::TaskStageChanged {
                                            task_id: session.task_id.clone(),
                                            task_json,
                                        });
                                    }
                                }
                                Err(e) => warn!(task_id = %session.task_id, error = %e, "Failed to fetch task after review update"),
                            }
                        }
                    }
                    Err(e) => warn!(session_id = %session_id_for_completion, error = %e, "Failed to fetch session for task review update"),
                }
            }

            // Post Claude's response as a comment
            if exit_ok {
                if let Some(text) = result_text {
                    if !text.is_empty() {
                        // We need the task_id — fetch it from the session record
                        if let Ok(session) = session_repo_for_completion.find(&session_id_for_completion).await {
                            use crate::models::CreateComment;
                            let content_len = text.len();
                            let preview = text.chars().take(120).collect::<String>();
                            let preview = if content_len > 120 { format!("{}…", preview) } else { preview };
                            match comment_repo_for_completion.create(
                                &session.task_id,
                                "claude",
                                CreateComment { content: text, parent_id: None },
                            ).await {
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
            self.session_repo.update(session_id, crate::models::UpdateSession {
                status: Some("stopped".to_string()),
                ended_at: Some(chrono::Utc::now()),
                ..Default::default()
            }).await?;

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

    pub async fn is_active(&self, session_id: &str) -> bool {
        self.active_sessions.read().await.contains_key(session_id)
    }

    pub async fn get_active_session_for_task(&self, task_id: &str) -> Option<String> {
        let sessions = self.active_sessions.read().await;
        sessions.iter()
            .find(|(_, rs)| rs.task.id == task_id)
            .map(|(session_id, _)| session_id.clone())
    }
}

fn count_project_files(project_path: &str) -> (i64, i64) {
    use std::fs;
    let mut file_count: i64 = 0;
    let mut loc: i64 = 0;

    fn visit_dir(path: &std::path::Path, file_count: &mut i64, loc: &mut i64) {
        let Ok(entries) = fs::read_dir(path) else { return };
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

    visit_dir(std::path::Path::new(project_path), &mut file_count, &mut loc);
    (file_count, loc)
}
