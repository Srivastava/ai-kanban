# Sprint 3: Real-time Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add WebSocket support for real-time session output streaming and task updates.

**Architecture:** WebSocket endpoint at `/ws` that broadcasts session output and task updates to connected clients. Uses broadcast channels from ClaudeManager.

**Tech Stack:** axum WebSocket, tokio broadcast channels, serde JSON

---

## Task 1: Create WebSocket Message Types

**Files:**
- Create: `backend/src/ws/mod.rs`
- Create: `backend/src/ws/messages.rs`

**Step 1: Create WebSocket module**

Create `backend/src/ws/mod.rs`:
```rust
mod messages;

pub use messages::*;
```

**Step 2: Create message types**

Create `backend/src/ws/messages.rs`:
```rust
use serde::{Deserialize, Serialize};

/// Messages from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe_task")]
    SubscribeTask { task_id: String },

    #[serde(rename = "subscribe_session")]
    SubscribeSession { session_id: String },

    #[serde(rename = "ping")]
    Ping,
}

/// Messages from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "task_updated")]
    TaskUpdated { task: serde_json::Value },

    #[serde(rename = "session_output")]
    SessionOutput {
        session_id: String,
        output: String,
        is_error: bool,
    },

    #[serde(rename = "session_status")]
    SessionStatus {
        session_id: String,
        status: String,
    },

    #[serde(rename = "token_update")]
    TokenUpdate {
        task_id: String,
        input_tokens: u64,
        output_tokens: u64,
    },

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "subscribed")]
    Subscribed { topic: String },
}

impl ServerMessage {
    pub fn session_output(session_id: String, output: String, is_error: bool) -> Self {
        ServerMessage::SessionOutput {
            session_id,
            output,
            is_error,
        }
    }

    pub fn session_status(session_id: String, status: String) -> Self {
        ServerMessage::SessionStatus { session_id, status }
    }

    pub fn error(message: impl Into<String>) -> Self {
        ServerMessage::Error {
            message: message.into(),
        }
    }

    pub fn subscribed(topic: impl Into<String>) -> Self {
        ServerMessage::Subscribed {
            topic: topic.into(),
        }
    }
}
```

**Step 3: Update lib.rs**

Add `pub mod ws;`

**Step 4: Verify compilation**

Run: `cd backend && cargo build`

**Step 5: Commit**

```bash
git add backend/src/
git commit -m "feat(ws): add WebSocket message types

- ClientMessage (subscribe_task, subscribe_session, ping)
- ServerMessage (task_updated, session_output, session_status, etc.)"
```

---

## Task 2: Create WebSocket Handler

**Files:**
- Create: `backend/src/ws/handler.rs`
- Modify: `backend/src/ws/mod.rs`

**Step 1: Create WebSocket handler**

Create `backend/src/ws/handler.rs`:
```rust
use crate::claude::{ClaudeManager, SessionOutput};
use crate::ws::{ClientMessage, ServerMessage};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    Extension,
};
use std::sync::Arc;
use tokio::sync::broadcast;
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

    // Task to send messages to client
    let send_task = async move {
        while let Ok(output) = output_rx.recv().await {
            let msg = ServerMessage::session_output(
                output.session_id,
                output.line,
                output.is_error,
            );

            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(json)).await.is_err() {
                debug!("Client disconnected");
                break;
            }
        }
    };

    // Task to receive messages from client
    let recv_task = async move {
        while let Some(msg) = receiver.recv().await {
            if let Ok(Message::Text(text)) = &msg {
                match serde_json::from_str::<ClientMessage>(text) {
                    Ok(client_msg) => {
                        debug!("Received: {:?}", client_msg);
                        // Handle subscriptions (for now just acknowledge)
                        match client_msg {
                            ClientMessage::Ping => {
                                // Pong is handled by the protocol
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse client message: {}", e);
                    }
                }
            }
        }
    };

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("WebSocket client disconnected");
}
```

**Step 2: Update ws/mod.rs**

```rust
mod handler;
mod messages;

pub use handler::ws_handler;
pub use messages::*;
```

**Step 3: Add WebSocket route**

Update `backend/src/api/routes.rs` to include WebSocket route.

**Step 4: Update main.rs**

Add ClaudeManager as Extension for WebSocket handler.

**Step 5: Verify compilation**

Run: `cd backend && cargo build`

**Step 6: Commit**

```bash
git add backend/src/
git commit -m "feat(ws): add WebSocket handler

- WebSocket endpoint at /ws
- Broadcast session output to connected clients
- Handle client subscriptions"
```

---

## Task 3: Add WebSocket Tests

**Files:**
- Create: `backend/tests/ws_test.rs`

**Step 1: Create WebSocket tests**

Create tests for:
- WebSocket connection
- Message serialization/deserialization
- Broadcast functionality

**Step 2: Run all tests**

Run: `cd backend && cargo test`

**Step 3: Commit**

```bash
git add backend/tests/
git commit -m "test: add WebSocket message tests

- ClientMessage/ServerMessage serialization
- Message parsing tests"
```

---

## Summary

Sprint 3 adds real-time capabilities:

- **WebSocket endpoint** at `/ws`
- **Session output streaming** in real-time
- **Message protocol** for client/server communication
- **Broadcast channel integration** with ClaudeManager

**Total Tests Target:** 110+ tests

---

## Post-Sprint: Test Verification

After Sprint 3, verify all tests pass:
```bash
cd backend && cargo test
```

Expected: All 110+ tests pass
