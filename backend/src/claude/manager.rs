use crate::claude::prompts::build_prompt;
use crate::db::SessionRepository;
use crate::models::{Session, Task};
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
}

impl ClaudeManager {
    pub fn new(session_repo: SessionRepository) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            session_repo,
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

        let mut child = Command::new("claude")
            .arg("--print")
            .arg(&prompt)
            .current_dir(&task.project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn Claude: {}", e))?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

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

        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(text) = line {
                    debug!(session_id = %session_id, "stdout: {}", text);
                    let _ = output_tx.send(SessionOutput {
                        session_id: session_id.clone(),
                        line: text,
                        is_error: false,
                    });
                }
            }
        });

        let session_id = session.id.clone();
        let output_tx = self.output_tx.clone();
        tokio::task::spawn_blocking(move || {
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

        Ok(session.id)
    }

    #[instrument(skip(self))]
    pub async fn stop_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(mut rs) = sessions.remove(session_id) {
            info!(session_id = %session_id, "Stopping session");
            let _ = rs.child.kill();
            self.session_repo.update(session_id, crate::models::UpdateSession {
                status: Some("paused".to_string()),
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
}
