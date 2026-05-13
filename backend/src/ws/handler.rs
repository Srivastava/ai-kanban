use crate::claude::{ClaudeEvent, ClaudeManager};
use crate::ws::{ClientMessage, ServerMessage};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    Extension,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, error, info, warn};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(manager): Extension<Arc<ClaudeManager>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, manager))
}

async fn handle_socket(socket: WebSocket, manager: Arc<ClaudeManager>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to session output
    let mut output_rx = manager.subscribe();

    info!("WebSocket client connected");

    // Shared state for which session this client is subscribed to
    let subscribed_session: Arc<tokio::sync::RwLock<Option<String>>> =
        Arc::new(tokio::sync::RwLock::new(None));
    let sub_send = subscribed_session.clone();

    // Channel for the recv task to inject direct-reply messages (e.g. Pong) into the send task.
    let (reply_tx, mut reply_rx) = tokio::sync::mpsc::channel::<ServerMessage>(8);

    // Task to send messages to client
    let mut send_task = tokio::spawn(async move {
        loop {
            let event = tokio::select! {
                // Direct replies (Pong, etc.) take priority
                Some(msg) = reply_rx.recv() => {
                    let json = match serde_json::to_string(&msg) {
                        Ok(j) => j,
                        Err(e) => { error!("Failed to serialize reply: {}", e); continue; }
                    };
                    if sender.send(Message::Text(json)).await.is_err() {
                        debug!("Client disconnected (reply send)");
                        break;
                    }
                    continue;
                }
                result = output_rx.recv() => result,
            };

            let event = match event {
                Ok(ev) => ev,
                Err(RecvError::Lagged(n)) => {
                    warn!("WS client lagged, dropped {} broadcast events", n);
                    // Notify the client that output was lost, then continue
                    let lag_msg = ServerMessage::error(format!(
                        "Output stream lagged: {} events were dropped. Reconnect to this session to see current state.",
                        n
                    ));
                    if let Ok(json) = serde_json::to_string(&lag_msg) {
                        let _ = sender.send(Message::Text(json)).await;
                    }
                    continue;
                }
                Err(RecvError::Closed) => break,
            };

            let msg: Option<ServerMessage> = {
                let sub = sub_send.read().await;
                let subscribed_id = sub.as_deref();

                match &event {
                    ClaudeEvent::Output {
                        session_id,
                        text,
                        is_error,
                    } => {
                        let should_send = subscribed_id.map_or(true, |id| id == session_id);
                        if should_send {
                            Some(ServerMessage::session_output(
                                session_id.clone(),
                                text.clone(),
                                *is_error,
                            ))
                        } else {
                            None
                        }
                    }
                    ClaudeEvent::Heartbeat {
                        session_id,
                        elapsed_secs,
                    } => {
                        let should_send = subscribed_id.map_or(true, |id| id == session_id);
                        if should_send {
                            Some(ServerMessage::session_heartbeat(
                                session_id.clone(),
                                *elapsed_secs,
                            ))
                        } else {
                            None
                        }
                    }
                    ClaudeEvent::SessionStatus { session_id, status } => {
                        let should_send = subscribed_id.map_or(true, |id| id == session_id);
                        if should_send {
                            Some(ServerMessage::session_status(
                                session_id.clone(),
                                status.clone(),
                            ))
                        } else {
                            None
                        }
                    }
                    ClaudeEvent::TaskStageChanged { task_json, .. } => {
                        // Broadcast to all — no session filter
                        Some(ServerMessage::TaskUpdated {
                            task: task_json.clone(),
                        })
                    }
                    ClaudeEvent::SessionIdAssigned {
                        session_id,
                        claude_session_id,
                    } => {
                        let should_send = subscribed_id.map_or(true, |id| id == session_id);
                        if should_send {
                            Some(ServerMessage::SessionIdAssigned {
                                session_id: session_id.clone(),
                                claude_session_id: claude_session_id.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    ClaudeEvent::RateLimited {
                        session_id,
                        task_id,
                        reset_at,
                        ..
                    } => {
                        let should_send = subscribed_id.map_or(true, |id| id == session_id);
                        if should_send {
                            Some(ServerMessage::RateLimited {
                                session_id: session_id.clone(),
                                task_id: task_id.clone(),
                                reset_at: reset_at.to_rfc3339(),
                            })
                        } else {
                            None
                        }
                    }
                    ClaudeEvent::StageContextSet {
                        session_id,
                        task_id,
                        mode,
                    } => {
                        // Broadcast to all clients — frontend filters by task_id
                        Some(ServerMessage::StageContextSet {
                            session_id: session_id.clone(),
                            task_id: task_id.clone(),
                            mode: mode.clone(),
                        })
                    }
                    ClaudeEvent::ContextFileUpdated {
                        session_id,
                        task_id,
                    } => {
                        // Broadcast to all clients — frontend filters by task_id
                        Some(ServerMessage::ContextFileUpdated {
                            session_id: session_id.clone(),
                            task_id: task_id.clone(),
                        })
                    }
                    ClaudeEvent::PlanCreated {
                        session_id,
                        task_id,
                        preview,
                    } => {
                        // Broadcast to all clients — frontend filters by task_id
                        Some(ServerMessage::PlanCreated {
                            session_id: session_id.clone(),
                            task_id: task_id.clone(),
                            preview: preview.clone(),
                        })
                    }
                    ClaudeEvent::EnrichmentStarted { task_id } => {
                        Some(ServerMessage::EnrichmentStarted {
                            task_id: task_id.clone(),
                        })
                    }
                    ClaudeEvent::EnrichmentCompleted { task_id } => {
                        Some(ServerMessage::EnrichmentCompleted {
                            task_id: task_id.clone(),
                        })
                    }
                    ClaudeEvent::EnrichmentFailed { task_id, error } => {
                        Some(ServerMessage::EnrichmentFailed {
                            task_id: task_id.clone(),
                            error: error.clone(),
                        })
                    }
                    ClaudeEvent::SummaryFailed {
                        session_id,
                        task_id,
                        error,
                    } => Some(ServerMessage::SummaryFailed {
                        session_id: session_id.clone(),
                        task_id: task_id.clone(),
                        error: error.clone(),
                    }),
                    ClaudeEvent::SessionFailed {
                        session_id,
                        task_id,
                        retry_attempt,
                        will_retry,
                        exit_code,
                        ..
                    } => {
                        // Broadcast to all — frontend filters by task_id
                        Some(ServerMessage::SessionFailed {
                            session_id: session_id.clone(),
                            task_id: task_id.clone(),
                            retry_attempt: *retry_attempt,
                            max_retries: 3u32,
                            will_retry: *will_retry,
                            exit_code: *exit_code,
                        })
                    }
                }
            };

            if let Some(msg) = msg {
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };
                if sender.send(Message::Text(json)).await.is_err() {
                    debug!("Client disconnected (send)");
                    break;
                }
            }
        }
    });

    // Task to receive messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = &msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::SubscribeSession { session_id }) => {
                        info!(session_id = %session_id, "Client subscribed to session");
                        let mut sub = subscribed_session.write().await;
                        *sub = Some(session_id);
                    }
                    Ok(ClientMessage::Ping) => {
                        debug!("Ping received");
                        let _ = reply_tx.send(ServerMessage::pong()).await;
                    }
                    Ok(client_msg) => {
                        debug!("Received: {:?}", client_msg);
                    }
                    Err(e) => {
                        warn!("Failed to parse client message: {}", e);
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        },
    }

    info!("WebSocket client disconnected");
}
