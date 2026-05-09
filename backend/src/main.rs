use ai_kanban_backend::ai::context_manager::ContextManager;
use ai_kanban_backend::ai::litellm::LitellmClient;
use ai_kanban_backend::api::{claude_usage_cli, create_router, otlp_router, AppState, OtlpState};
use ai_kanban_backend::claude::ClaudeManager;
use ai_kanban_backend::db::{
    create_pool, AttachmentRepository, CommentRepository, LogRepository, OtelLogsRepository,
    OtelMetricsRepository, SessionMetricsRepository, SessionRepository, SettingsRepository,
    TaskRepository, TokenEventRepository,
};
use ai_kanban_backend::logging::DbLayer;
use axum::Extension;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/ai-kanban.db".into());
    let pool = create_pool(&db_path).await?;

    // Create repositories
    let task_repo = TaskRepository::new(pool.clone());
    let log_repo = LogRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let comment_repo = CommentRepository::new(pool.clone());
    let token_event_repo = TokenEventRepository::new(pool.clone());
    let session_metrics_repo = SessionMetricsRepository::new(pool.clone());
    let otel_repo = OtelMetricsRepository::new(pool.clone());
    let otel_logs_repo = OtelLogsRepository::new(pool.clone());
    let settings_repo = SettingsRepository::new(pool.clone());
    let attachment_repo = AttachmentRepository::new(pool.clone());

    // OTLP receiver on port 4318
    let otlp_state = OtlpState {
        otel_repo: otel_repo.clone(),
        otel_logs_repo: otel_logs_repo.clone(),
        session_repo: session_repo.clone(),
    };
    let otlp_app = otlp_router(otlp_state).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );
    let otlp_addr = std::net::SocketAddr::from(([0, 0, 0, 0], 4318));
    let otlp_listener = tokio::net::TcpListener::bind(otlp_addr).await?;
    tracing::info!("OTLP receiver listening on {}", otlp_addr);
    tokio::spawn(async move {
        axum::serve(otlp_listener, otlp_app)
            .await
            .expect("OTLP server failed");
    });

    // Initialize context manager (LiteLLM-backed summarization)
    let litellm = LitellmClient::from_env();
    tracing::info!(
        base_url = %litellm.base_url,
        model = %litellm.model,
        "LiteLLM context manager configured"
    );
    let context_manager = Arc::new(ContextManager::new(
        litellm,
        comment_repo.clone(),
        task_repo.clone(),
        attachment_repo.clone(),
    ));

    // Initialize Claude manager and session queue
    let claude_manager = Arc::new(ClaudeManager::new(
        session_repo.clone(),
        token_event_repo.clone(),
        session_metrics_repo.clone(),
        comment_repo.clone(),
        task_repo.clone(),
        otel_repo.clone(),
        Some(context_manager),
        Some(settings_repo.clone()),
        attachment_repo.clone(),
    ));
    let queue = Arc::new(ai_kanban_backend::claude::SessionQueue::new(
        claude_manager.clone(),
        task_repo.clone(),
    ));

    // Initialize logging with DB layer
    let db_layer = DbLayer::new(log_repo.clone());
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(db_layer)
        .init();

    tracing::info!("Database initialized at {}", db_path);
    tracing::info!("Logging system initialized");

    // Startup recovery: sessions left in "pending" or "running" state from a previous
    // backend instance are orphaned — their processes no longer exist.
    // Mark them stopped so the UI doesn't show them as stuck indefinitely.
    {
        let mut orphaned = session_repo
            .list_by_status("pending")
            .await
            .unwrap_or_default();
        orphaned.extend(
            session_repo
                .list_by_status("running")
                .await
                .unwrap_or_default(),
        );
        if !orphaned.is_empty() {
            tracing::warn!(
                count = orphaned.len(),
                "Recovering orphaned sessions from prior run"
            );
            for s in &orphaned {
                let _ = session_repo
                    .update(
                        &s.id,
                        ai_kanban_backend::models::UpdateSession {
                            status: Some("stopped".to_string()),
                            error_message: Some(
                                "Session orphaned — backend restarted while session was active"
                                    .to_string(),
                            ),
                            ended_at: Some(chrono::Utc::now()),
                            ..Default::default()
                        },
                    )
                    .await;
            }
            tracing::info!(count = orphaned.len(), "Orphaned sessions marked as stopped");
        }
    }

    // Session-completion listener: advances the queue when a session ends naturally or fails.
    // Manual stop is handled directly in sessions.rs; rate-limit is handled below.
    {
        let mut event_rx = claude_manager.subscribe();
        let queue_for_completion = queue.clone();
        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(ai_kanban_backend::claude::ClaudeEvent::SessionStatus {
                        session_id,
                        status,
                    }) => {
                        if status == "completed" || status == "failed" {
                            if let Err(e) =
                                queue_for_completion.on_session_complete(&session_id).await
                            {
                                tracing::error!(
                                    session_id = %session_id,
                                    status = %status,
                                    error = %e,
                                    "Queue failed to advance after session completed"
                                );
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    _ => {}
                }
            }
        });
    }

    // Rate-limit listener: schedules retry for the rate-limited task AND advances
    // the queue so other pending tasks can use the freed slot immediately.
    {
        let mut event_rx = claude_manager.subscribe();
        let queue_for_rl = queue.clone();
        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(ai_kanban_backend::claude::ClaudeEvent::RateLimited {
                        session_id,
                        task_id,
                        stage,
                        claude_session_id,
                        reset_at,
                    }) => {
                        // Advance queue for other waiting tasks (slot is now free)
                        if let Err(e) = queue_for_rl.on_session_complete(&session_id).await {
                            tracing::warn!(session_id = %session_id, error = %e, "Queue advance after rate-limit failed");
                        }
                        // Schedule retry for the rate-limited task itself
                        queue_for_rl
                            .clone()
                            .schedule_rate_limit_retry(task_id, stage, claude_session_id, reset_at)
                            .await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    _ => {}
                }
            }
        });
    }

    // Start the usage-polling daemon once, here in main, so it is not accidentally
    // spawned multiple times if AppState is ever cloned into multiple router states.
    let usage_cache = claude_usage_cli::start_usage_daemon(Some(queue.clone()));

    // Watchdog: every 5 minutes, find sessions that are "running" in the DB but no longer
    // in the in-memory active map (zombies from crashes/restarts) and mark them stopped.
    {
        let manager_watchdog = claude_manager.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(300));
            interval.tick().await; // skip the immediate first tick
            loop {
                interval.tick().await;
                match manager_watchdog.reconcile_zombie_sessions().await {
                    Ok(0) => {}
                    Ok(n) => tracing::warn!(count = n, "Watchdog: zombie sessions recovered"),
                    Err(e) => {
                        tracing::error!(error = %e, "Watchdog: session reconciliation failed")
                    }
                }
            }
        });
    }

    // Create state with queue
    let state = AppState::new(
        task_repo,
        log_repo,
        session_repo,
        comment_repo,
        token_event_repo,
        session_metrics_repo,
        settings_repo,
        otel_repo,
        attachment_repo,
        usage_cache,
    )
    .with_queue(queue)
    .with_pool(pool);
    tracing::debug!("Application state created");

    // Build app with CORS and Extension for WebSocket
    let app = create_router(state)
        .layer(Extension(claude_manager.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    tracing::info!("Server shutdown");
    Ok(())
}
