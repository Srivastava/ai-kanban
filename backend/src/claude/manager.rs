use crate::claude::jsonl_parser::{extract_result_text, parse_jsonl_line};
use crate::claude::prompts::build_prompt;
use crate::db::{CommentRepository, SessionMetricsRepository, SessionRepository, TaskRepository, TokenEventRepository};
use crate::models::{CreateTokenEvent, Session, Task, UpdateSession, UpdateTask};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, instrument, warn};

#[derive(Debug, Clone)]
pub struct SessionOutput {
    pub session_id: String,
    pub line: String,
    pub is_error: bool,
}

struct RunningSession {
    session: Session,
    child: Child,
    task: Task,
}

pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,
    output_tx: broadcast::Sender<SessionOutput>,
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

    pub fn subscribe(&self) -> broadcast::Receiver<SessionOutput> {
        self.output_tx.subscribe()
    }

    #[instrument(skip(self, task))]
    pub async fn start_session(&self, task: Task, stage: &str) -> Result<String> {
        let session = self.session_repo.create(crate::models::CreateSession {
            task_id: task.id.clone(),
        }).await?;

        info!(session_id = %session.id, task_id = %task.id, "Starting Claude session");

        let prompt = build_prompt(&task.title, task.description.as_deref(), stage);

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

        let mut child = Command::new(&claude_bin)
            .arg("--print")
            .arg("--verbose")
            .arg("--dangerously-skip-permissions")
            .arg("--output-format").arg("stream-json")
            .arg(&prompt)
            .current_dir(&project_path)
            // Unset CLAUDECODE so the CLI doesn't refuse to run inside another Claude session
            .env_remove("CLAUDECODE")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn Claude (bin={}): {}", claude_bin, e))?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

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

        // Link session to task
        let task_repo_link = self.task_repo.clone();
        let task_id_link = task_id.clone();
        let session_id_link = session.id.clone();
        tokio::spawn(async move {
            if let Err(e) = task_repo_link.update(&task_id_link, UpdateTask {
                session_id: Some(session_id_link),
                ..Default::default()
            }).await {
                warn!(task_id = %task_id_link, error = %e, "Failed to link session_id to task");
            }
        });

        // Snapshot project metrics at session start
        let project_metrics = count_project_files(&task_id);
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
        let stdout_handle = tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            let mut sequence_no: i64 = 0;
            let mut result_text: Option<String> = None;
            for line in reader.lines() {
                if let Ok(text) = line {
                    debug!(session_id = %session_id, "stdout: {}", text);

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

                    let _ = output_tx.send(SessionOutput {
                        session_id: session_id.clone(),
                        line: text,
                        is_error: false,
                    });
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
                    let _ = output_tx.send(SessionOutput {
                        session_id: session_id.clone(),
                        line: text,
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
                        info!(session_id = %session_id_for_completion, exit_code = ?status.code(), "Claude process exited");
                        status.success()
                    }
                    _ => false,
                }
            } else {
                // Session was already removed (e.g. manually stopped)
                return;
            };

            let final_status = if exit_ok { "completed" } else { "failed" };
            info!(session_id = %session_id_for_completion, status = %final_status, "Marking session complete");
            let _ = session_repo_for_completion.update(&session_id_for_completion, UpdateSession {
                status: Some(final_status.to_string()),
                ended_at: Some(chrono::Utc::now()),
                ..Default::default()
            }).await;

            // Post Claude's response as a comment
            if exit_ok {
                if let Some(text) = result_text {
                    if !text.is_empty() {
                        // We need the task_id — fetch it from the session record
                        if let Ok(session) = session_repo_for_completion.find(&session_id_for_completion).await {
                            use crate::models::CreateComment;
                            let _ = comment_repo_for_completion.create(
                                &session.task_id,
                                "claude",
                                CreateComment { content: text, parent_id: None },
                            ).await;
                            info!(session_id = %session_id_for_completion, "Posted Claude result as comment");
                        }
                    }
                }
            }
        });

        Ok(session.id)
    }

    #[instrument(skip(self))]
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(mut rs) = sessions.remove(session_id) {
            info!(session_id = %session_id, "Stopping session");
            let _ = rs.child.kill();
            self.session_repo.update(session_id, crate::models::UpdateSession {
                status: Some("stopped".to_string()),
                ended_at: Some(chrono::Utc::now()),
                ..Default::default()
            }).await?;
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
