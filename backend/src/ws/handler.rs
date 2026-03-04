use crate::claude::{ClaudeEvent, ClaudeManager};
use crate::ws::{ClientMessage, ServerMessage};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    Extension,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
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

    // Task to send messages to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = output_rx.recv().await {
            let msg: Option<ServerMessage> = {
                let sub = sub_send.read().await;
                let subscribed_id = sub.as_deref();

                match &event {
                    ClaudeEvent::Output { session_id, text, is_error } => {
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
                    ClaudeEvent::Heartbeat { session_id, elapsed_secs } => {
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
                        Some(ServerMessage::TaskUpdated { task: task_json.clone() })
                    }
                    ClaudeEvent::SessionIdAssigned { session_id, claude_session_id } => {
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
                    ClaudeEvent::RateLimited { session_id, task_id, reset_at, .. } => {
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
