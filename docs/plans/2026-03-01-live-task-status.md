# Live Task Status + Stage Progression Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the Live Output Panel (show human-readable output), add a running heartbeat indicator, and auto-advance task stage as Claude works (planning → in_progress → review).

**Architecture:** Replace the narrow `SessionOutput` broadcast with a typed `ClaudeEvent` enum that carries parsed output, heartbeats, session status, and task stage changes. Manager emits events; the WS handler routes them to clients; the frontend reacts in real-time.

**Tech Stack:** Rust/Axum backend, tokio broadcast channels, React/Next.js frontend, TanStack Query, WebSocket context

---

## Task 1: Add `ClaudeEvent` enum and export it

**Files:**
- Modify: `backend/src/claude/manager.rs`
- Modify: `backend/src/claude/mod.rs`

### Step 1: Add `ClaudeEvent` enum to `manager.rs`

In `backend/src/claude/manager.rs`, replace the `SessionOutput` struct with a `ClaudeEvent` enum. Find the existing `SessionOutput` definition at lines 13-18 and replace it:

```rust
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
}
```

Also update `ClaudeManager` to use `broadcast::Sender<ClaudeEvent>` instead of `broadcast::Sender<SessionOutput>`. The field `output_tx` changes type:

```rust
pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,
    output_tx: broadcast::Sender<ClaudeEvent>,
    // ... rest unchanged
}
```

And in `new()`:
```rust
let (output_tx, _) = broadcast::channel(1024);
```

And `subscribe()`:
```rust
pub fn subscribe(&self) -> broadcast::Receiver<ClaudeEvent> {
    self.output_tx.subscribe()
}
```

### Step 2: Update `claude/mod.rs` exports

In `backend/src/claude/mod.rs`, change the pub use line:

```rust
pub use manager::{ClaudeManager, ClaudeEvent};
```

Remove `SessionOutput` from the export (it no longer exists).

### Step 3: Build to verify it compiles (with errors expected in dependent files)

Run: `cd backend && cargo build 2>&1 | head -40`

Expected: compile errors in `ws/handler.rs` and within `manager.rs` where `SessionOutput` is still used — that's fine, we'll fix those in subsequent tasks.

### Step 4: Commit stub

```bash
git add backend/src/claude/manager.rs backend/src/claude/mod.rs
git commit -m "refactor: replace SessionOutput with ClaudeEvent enum"
```

---

## Task 2: Add `parse_for_display()` to `jsonl_parser.rs`

**Files:**
- Modify: `backend/src/claude/jsonl_parser.rs`
- Modify: `backend/tests/jsonl_parser_test.rs`

### Step 1: Write the failing tests first

Add to the bottom of `backend/tests/jsonl_parser_test.rs`:

```rust
use ai_kanban_backend::claude::jsonl_parser::parse_for_display;

#[test]
fn test_display_read_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/main.rs"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("📖 Read: src/main.rs".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_write_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"frontend/src/app/page.tsx"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("✏️ Write: frontend/src/app/page.tsx".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_edit_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/lib.rs"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("✏️ Edit: src/lib.rs".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_bash_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test 2>&1"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("⚡ Bash: cargo test 2>&1".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_glob_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Glob","input":{"pattern":"**/*.rs"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("🔍 Glob: **/*.rs".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_grep_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Grep","input":{"pattern":"fn parse"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("🔍 Grep: fn parse".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_other_tool() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Agent","input":{"description":"explore codebase"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("🔧 Agent: explore codebase".to_string()));
    assert!(has_tool);
}

#[test]
fn test_display_assistant_text() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"I'll start by reading the existing code to understand the structure."}],"usage":{"input_tokens":100,"output_tokens":20}}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("🤔 I'll start by reading the existing code to understand the structure.".to_string()));
    assert!(!has_tool);
}

#[test]
fn test_display_assistant_text_truncated() {
    let long_text = "a".repeat(200);
    let line = format!(r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"{}"}}],"usage":{{"input_tokens":100,"output_tokens":20}}}}}}"#, long_text);
    let (text, _) = parse_for_display(&line);
    let text = text.unwrap();
    assert!(text.starts_with("🤔 "));
    // Should be truncated — max 120 chars of content + prefix + ellipsis
    assert!(text.len() <= 130);
    assert!(text.ends_with("..."));
}

#[test]
fn test_display_result_success() {
    let line = r#"{"type":"result","subtype":"success","result":"Done","usage":{"input_tokens":100,"output_tokens":10}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("✅ Session complete".to_string()));
    assert!(!has_tool);
}

#[test]
fn test_display_result_error() {
    let line = r#"{"type":"result","subtype":"error","error":"Something went wrong","usage":{"input_tokens":100,"output_tokens":10}}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, Some("❌ Error: Something went wrong".to_string()));
    assert!(!has_tool);
}

#[test]
fn test_display_system_skipped() {
    let line = r#"{"type":"system","subtype":"init","model":"claude-sonnet-4-6"}"#;
    let (text, has_tool) = parse_for_display(line);
    assert_eq!(text, None);
    assert!(!has_tool);
}

#[test]
fn test_display_plain_text_skipped() {
    let (text, has_tool) = parse_for_display("Not JSON at all");
    assert_eq!(text, None);
    assert!(!has_tool);
}
```

### Step 2: Run tests to verify they fail

Run: `cd backend && cargo test test_display --test jsonl_parser_test 2>&1 | head -20`

Expected: compile error — `parse_for_display` not found.

### Step 3: Implement `parse_for_display()` in `jsonl_parser.rs`

Add after the existing `extract_result_text()` function:

```rust
/// Parse a JSONL line into a human-readable display string and whether a tool_use was found.
/// Returns (Option<display_text>, has_tool_use).
/// Returns (None, false) for lines that should be skipped (system events, non-JSON, etc).
pub fn parse_for_display(line: &str) -> (Option<String>, bool) {
    let value: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return (None, false),
    };

    match value.get("type").and_then(|t| t.as_str()) {
        Some("assistant") => parse_assistant_for_display(&value),
        Some("result") => {
            let text = match value.get("subtype").and_then(|s| s.as_str()) {
                Some("success") => Some("✅ Session complete".to_string()),
                _ => {
                    let msg = value
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("unknown error");
                    Some(format!("❌ Error: {}", msg))
                }
            };
            (text, false)
        }
        _ => (None, false), // system, tool results, unknown — skip
    }
}

fn parse_assistant_for_display(value: &serde_json::Value) -> (Option<String>, bool) {
    let message = match value.get("message") {
        Some(m) => m,
        None => return (None, false),
    };
    let content = match message.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return (None, false),
    };

    // Look for tool_use first (takes priority over text)
    for item in content {
        if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
            let name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("Unknown");
            let input = item.get("input");
            let text = format_tool_display(name, input);
            return (Some(text), true);
        }
    }

    // Fall back to text content
    for item in content {
        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if !text.trim().is_empty() {
                    let truncated = if text.len() > 120 {
                        format!("{}...", &text[..120])
                    } else {
                        text.to_string()
                    };
                    return (Some(format!("🤔 {}", truncated)), false);
                }
            }
        }
    }

    (None, false)
}

fn format_tool_display(name: &str, input: Option<&serde_json::Value>) -> String {
    match name {
        "Read" => {
            let path = get_input_path(input).unwrap_or_default();
            format!("📖 Read: {}", path)
        }
        "Write" | "Edit" | "NotebookEdit" => {
            let path = get_input_path(input).unwrap_or_default();
            format!("✏️ {}: {}", name, path)
        }
        "Bash" => {
            let cmd = input
                .and_then(|i| i.get("command"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let preview = if cmd.len() > 80 { &cmd[..80] } else { cmd };
            format!("⚡ Bash: {}", preview)
        }
        "Glob" => {
            let pattern = input
                .and_then(|i| i.get("pattern"))
                .and_then(|p| p.as_str())
                .unwrap_or("");
            format!("🔍 Glob: {}", pattern)
        }
        "Grep" => {
            let pattern = input
                .and_then(|i| i.get("pattern"))
                .and_then(|p| p.as_str())
                .unwrap_or("");
            format!("🔍 Grep: {}", pattern)
        }
        _ => {
            // Generic: show first string value from input
            let arg = get_first_string_value(input).unwrap_or_default();
            format!("🔧 {}: {}", name, arg)
        }
    }
}

fn get_input_path(input: Option<&serde_json::Value>) -> Option<String> {
    let input = input?;
    input
        .get("file_path")
        .or_else(|| input.get("path"))
        .or_else(|| input.get("notebook_path"))
        .and_then(|p| p.as_str())
        .map(|s| s.to_string())
}

fn get_first_string_value(input: Option<&serde_json::Value>) -> Option<String> {
    let obj = input?.as_object()?;
    for (_, v) in obj {
        if let Some(s) = v.as_str() {
            if !s.is_empty() {
                let preview = if s.len() > 80 { &s[..80] } else { s };
                return Some(preview.to_string());
            }
        }
    }
    None
}
```

### Step 4: Run tests to verify they pass

Run: `cd backend && cargo test test_display --test jsonl_parser_test 2>&1`

Expected: All 14 display tests pass.

### Step 5: Run full backend test suite

Run: `cd backend && cargo test 2>&1`

Expected: All existing tests still pass plus the new display tests.

### Step 6: Commit

```bash
git add backend/src/claude/jsonl_parser.rs backend/tests/jsonl_parser_test.rs
git commit -m "feat: add parse_for_display() for human-readable JSONL output"
```

---

## Task 3: Update `ws/messages.rs` — add `SessionHeartbeat`

**Files:**
- Modify: `backend/src/ws/messages.rs`
- Modify: `backend/tests/ws_test.rs`

### Step 1: Write the failing test

Add to `backend/tests/ws_test.rs`:

```rust
#[test]
fn test_server_message_session_heartbeat() {
    let msg = ServerMessage::SessionHeartbeat {
        session_id: "session-abc".to_string(),
        elapsed_secs: 42,
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"session_heartbeat\""));
    assert!(json.contains("\"session_id\":\"session-abc\""));
    assert!(json.contains("\"elapsed_secs\":42"));
}
```

### Step 2: Run to verify failure

Run: `cd backend && cargo test test_server_message_session_heartbeat --test ws_test 2>&1`

Expected: compile error — `SessionHeartbeat` variant doesn't exist yet.

### Step 3: Add the variant to `messages.rs`

In `backend/src/ws/messages.rs`, add to the `ServerMessage` enum after the `SessionStatus` variant:

```rust
#[serde(rename = "session_heartbeat")]
SessionHeartbeat {
    session_id: String,
    elapsed_secs: u64,
},
```

Also add a constructor:

```rust
pub fn session_heartbeat(session_id: String, elapsed_secs: u64) -> Self {
    ServerMessage::SessionHeartbeat { session_id, elapsed_secs }
}
```

### Step 4: Run tests to verify they pass

Run: `cd backend && cargo test --test ws_test 2>&1`

Expected: All ws_test tests pass.

### Step 5: Commit

```bash
git add backend/src/ws/messages.rs backend/tests/ws_test.rs
git commit -m "feat: add SessionHeartbeat WS message type"
```

---

## Task 4: Update `manager.rs` — emit `ClaudeEvent`s, stage progression, heartbeat

This is the largest task. The `manager.rs` stdout processing loop needs to:
1. Replace all `SessionOutput` sends with `ClaudeEvent::Output` sends (using `parse_for_display`)
2. Track `first_tool_seen` and emit `TaskStageChanged` when first tool is detected
3. Emit `TaskStageChanged(planning)` at session start
4. Emit heartbeat every 5s in a background task
5. Emit `TaskStageChanged(review)` and `SessionStatus(completed)` at completion

**Files:**
- Modify: `backend/src/claude/manager.rs`

### Step 1: Add required imports to `manager.rs`

At the top of `manager.rs`, ensure these imports are present (add if missing):

```rust
use std::time::Instant;
use tokio::time::Duration;
```

### Step 2: Fix `start_session` — update task stage to "planning" at start

In the `start_session` function, after linking session to task (after line 135 in the original), add a block to update the task stage to "planning":

```rust
// Advance task to planning stage
let task_repo_stage = self.task_repo.clone();
let task_id_stage = task_id.clone();
let output_tx_stage = self.output_tx.clone();
tokio::spawn(async move {
    if let Err(e) = task_repo_stage.update(&task_id_stage, UpdateTask {
        stage: Some("planning".to_string()),
        ..Default::default()
    }).await {
        warn!(task_id = %task_id_stage, error = %e, "Failed to set task stage to planning");
        return;
    }
    if let Ok(task) = task_repo_stage.find(&task_id_stage).await {
        if let Ok(task_json) = serde_json::to_value(&task) {
            let _ = output_tx_stage.send(ClaudeEvent::TaskStageChanged {
                task_id: task_id_stage.clone(),
                task_json,
            });
        }
    }
});
```

Note: `task_repo.find()` — check `backend/src/db/tasks.rs` for the correct method name. It should be `.find()` or `.get_by_id()`. Use the same method used elsewhere in manager.rs.

### Step 3: Replace stdout processing to use `parse_for_display` and track first tool use

Find the `stdout_handle` spawn_blocking block (lines ~150-192). Replace the inner for loop logic:

```rust
let stdout_handle = tokio::task::spawn_blocking(move || {
    let reader = BufReader::new(stdout);
    let mut sequence_no: i64 = 0;
    let mut result_text: Option<String> = None;
    let mut first_tool_seen = false;

    for line in reader.lines() {
        if let Ok(text) = line {
            debug!(session_id = %session_id, "stdout: {}", text);

            // Parse for token event recording (existing logic, unchanged)
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

            // Extract final result text (existing logic, unchanged)
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
                    if let Ok(task) = task_repo_tool.find(&task_id_tool).await {
                        if let Ok(task_json) = serde_json::to_value(&task) {
                            let _ = output_tx_tool.send(ClaudeEvent::TaskStageChanged {
                                task_id: task_id_tool.clone(),
                                task_json,
                            });
                        }
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
```

You will need to capture two additional variables before this spawn_blocking:
```rust
let task_repo_for_stage = self.task_repo.clone();
let output_tx_for_stage = self.output_tx.clone();
```
Add these just before the `stdout_handle = tokio::task::spawn_blocking(...)` call.

Also update stderr to use `ClaudeEvent::Output`:
```rust
let _ = output_tx.send(ClaudeEvent::Output {
    session_id: session_id.clone(),
    text: text,
    is_error: true,
});
```

### Step 4: Add heartbeat task

After the `active_sessions` insert (after the `sessions.insert(...)` call), spawn the heartbeat loop:

```rust
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
```

### Step 5: Update completion handler — emit `TaskStageChanged(review)` and `SessionStatus`

In the completion `tokio::spawn` block, after updating session status, add stage progression and WS events. Find where `final_status` is determined and the session update happens. Replace/extend that section:

```rust
let final_status = if exit_ok { "completed" } else { "failed" };
info!(
    session_id = %session_id_for_completion,
    status = %final_status,
    "Session finished"
);
let _ = session_repo_for_completion.update(&session_id_for_completion, UpdateSession {
    status: Some(final_status.to_string()),
    ended_at: Some(chrono::Utc::now()),
    ..Default::default()
}).await;

// Emit session status via WS
let _ = output_tx_for_completion.send(ClaudeEvent::SessionStatus {
    session_id: session_id_for_completion.clone(),
    status: final_status.to_string(),
});

// On success: advance task to review
if exit_ok {
    if let Ok(session) = session_repo_for_completion.find(&session_id_for_completion).await {
        if let Err(e) = task_repo_for_completion.update(&session.task_id, UpdateTask {
            stage: Some("review".to_string()),
            ..Default::default()
        }).await {
            warn!(task_id = %session.task_id, error = %e, "Failed to set task stage to review");
        } else if let Ok(task) = task_repo_for_completion.find(&session.task_id).await {
            if let Ok(task_json) = serde_json::to_value(&task) {
                let _ = output_tx_for_completion.send(ClaudeEvent::TaskStageChanged {
                    task_id: session.task_id.clone(),
                    task_json,
                });
            }
        }
    }
}
```

You need to capture two more variables before the completion `tokio::spawn`. Add them alongside the existing ones:

```rust
let output_tx_for_completion = self.output_tx.clone();
let task_repo_for_completion = self.task_repo.clone();
```

### Step 6: Remove the old `SessionOutput` import and fix `use` statement

Remove `SessionOutput` from any `use` statement in `manager.rs` since it no longer exists. The `ClaudeEvent` is defined in the same file so no import needed.

### Step 7: Check `task_repo` has a `find()` method

Run: `grep -n "pub async fn find\|pub async fn get" backend/src/db/tasks.rs`

If the method is named differently (e.g., `get_by_id`), use that name consistently in all the new code in steps 2, 3, and 5.

### Step 8: Build to verify it compiles

Run: `cd backend && cargo build 2>&1`

Expected: `ws/handler.rs` will still have compile errors (uses old `SessionOutput` type) — that's expected. `manager.rs` itself should compile cleanly.

### Step 9: Commit

```bash
git add backend/src/claude/manager.rs
git commit -m "feat: emit ClaudeEvents with stage progression and heartbeat"
```

---

## Task 5: Update `ws/handler.rs` to route `ClaudeEvent` to `ServerMessage`

**Files:**
- Modify: `backend/src/ws/handler.rs`

### Step 1: Update the send task to map `ClaudeEvent` variants

The `send_task` loop currently accesses `output.session_id` directly. Replace it to match on the enum:

```rust
let mut send_task = tokio::spawn(async move {
    while let Ok(event) = output_rx.recv().await {
        // Determine if this event should be sent to this client
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
                    // Broadcast to all connected clients (no session filter)
                    Some(ServerMessage::TaskUpdated { task: task_json.clone() })
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
```

Also add the import at the top of `handler.rs`:

```rust
use crate::claude::ClaudeEvent;
```

### Step 2: Build to verify it compiles

Run: `cd backend && cargo build 2>&1`

Expected: Clean build — no errors.

### Step 3: Run all backend tests

Run: `cd backend && cargo test 2>&1`

Expected: All tests pass. (The ws_test.rs tests don't test handler.rs directly, but the build passing confirms the types are correct.)

### Step 4: Commit

```bash
git add backend/src/ws/handler.rs
git commit -m "feat: route ClaudeEvent variants to WS ServerMessage"
```

---

## Task 6: Update `live-output-panel.tsx` — heartbeat indicator

**Files:**
- Modify: `frontend/src/components/sessions/live-output-panel.tsx`

### Step 1: Add heartbeat state and subscription

The component needs to:
1. Subscribe to `session_heartbeat` messages and track last heartbeat
2. Show "Running Xs" using `elapsed_secs` from the most recent heartbeat
3. If >8s since last heartbeat message arrived, show "Waiting..."

Replace the full file content with:

```tsx
'use client';

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '@/contexts/websocket-context';
import type { SessionStatus } from '@/types/session';

interface Props {
  sessionId: string;
  status: SessionStatus | null | undefined;
}

interface OutputLine {
  text: string;
  isError: boolean;
}

interface HeartbeatState {
  elapsedSecs: number;
  receivedAt: number; // Date.now() when we got it
}

export function LiveOutputPanel({ sessionId, status }: Props) {
  const [lines, setLines] = useState<OutputLine[]>([]);
  const [heartbeat, setHeartbeat] = useState<HeartbeatState | null>(null);
  const [displayElapsed, setDisplayElapsed] = useState<number>(0);
  const bottomRef = useRef<HTMLDivElement>(null);
  const { subscribe, send, status: wsStatus } = useWebSocket();
  const isConnected = wsStatus === 'connected';

  // Reset lines and heartbeat when session changes
  useEffect(() => {
    setLines([]);
    setHeartbeat(null);
  }, [sessionId]);

  // Subscribe to this session's output
  useEffect(() => {
    if (!sessionId || !isConnected) return;

    send({ type: 'subscribe_session', session_id: sessionId });

    const unsubOutput = subscribe('session_output', (data: unknown) => {
      const msg = data as { session_id: string; output: string; is_error: boolean };
      if (msg.session_id !== sessionId) return;
      setLines((prev) => [
        ...prev.slice(-500),
        { text: msg.output, isError: msg.is_error },
      ]);
    });

    const unsubHeartbeat = subscribe('session_heartbeat', (data: unknown) => {
      const msg = data as { session_id: string; elapsed_secs: number };
      if (msg.session_id !== sessionId) return;
      setHeartbeat({ elapsedSecs: msg.elapsed_secs, receivedAt: Date.now() });
    });

    return () => {
      unsubOutput();
      unsubHeartbeat();
    };
  }, [sessionId, isConnected, subscribe, send]);

  // Tick display elapsed every second while running
  useEffect(() => {
    const isRunning = status === 'running' || status === 'pending';
    if (!isRunning || !heartbeat) return;

    const interval = setInterval(() => {
      const secsSinceHeartbeat = Math.floor((Date.now() - heartbeat.receivedAt) / 1000);
      setDisplayElapsed(heartbeat.elapsedSecs + secsSinceHeartbeat);
    }, 1000);

    // Set immediately too
    const secsSinceHeartbeat = Math.floor((Date.now() - heartbeat.receivedAt) / 1000);
    setDisplayElapsed(heartbeat.elapsedSecs + secsSinceHeartbeat);

    return () => clearInterval(interval);
  }, [heartbeat, status]);

  // Auto-scroll to bottom
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [lines]);

  if (!sessionId) return null;

  const isRunning = status === 'running' || status === 'pending';
  const secsSinceHeartbeat = heartbeat ? Math.floor((Date.now() - heartbeat.receivedAt) / 1000) : 999;
  const isWaiting = isRunning && heartbeat && secsSinceHeartbeat > 8;
  const hasHeartbeat = isRunning && heartbeat && !isWaiting;

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-muted/40 border-b border-border">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Session Output
        </span>
        {hasHeartbeat && (
          <span className="flex items-center gap-1.5 text-xs text-emerald-500">
            <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
            Running {displayElapsed}s
          </span>
        )}
        {isWaiting && (
          <span className="flex items-center gap-1.5 text-xs text-yellow-500">
            <span className="h-2 w-2 rounded-full bg-yellow-500 animate-pulse" />
            Waiting...
          </span>
        )}
        {isRunning && !heartbeat && (
          <span className="flex items-center gap-1.5 text-xs text-emerald-500">
            <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse" />
            Live
          </span>
        )}
        {!isRunning && lines.length > 0 && (
          <span className="text-xs text-muted-foreground">Completed</span>
        )}
      </div>
      <div className="max-h-64 overflow-y-auto bg-black/90 p-3 font-mono text-xs">
        {lines.length === 0 ? (
          <p className="text-muted-foreground italic">
            {isRunning ? 'Waiting for output...' : 'No output captured.'}
          </p>
        ) : (
          lines.map((line, i) => (
            <div
              key={i}
              className={`whitespace-pre-wrap ${line.isError ? 'text-red-400' : 'text-green-300/90'}`}
            >
              {line.text}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
```

### Step 2: Verify TypeScript compiles

Run: `cd frontend && npx tsc --noEmit 2>&1`

Expected: No errors.

### Step 3: Commit

```bash
git add frontend/src/components/sessions/live-output-panel.tsx
git commit -m "feat: add heartbeat indicator to LiveOutputPanel"
```

---

## Task 7: Add global `task_updated` WS handler to update task query cache

**Files:**
- Modify: `frontend/src/contexts/websocket-context.tsx`

### Step 1: Add `queryClient` and `task_updated` subscription

The `WebSocketProvider` needs access to `useQueryClient`. Since React Query context must be inside a `QueryClientProvider`, and `WebSocketProvider` is presumably also inside it (check `app/layout.tsx` or `app/providers.tsx` to confirm the nesting order), this should work.

In `websocket-context.tsx`, add the import:

```tsx
import { useQueryClient } from '@tanstack/react-query';
import type { Task } from '@/types/task';
```

Inside `WebSocketProvider`, at the top of the component body, add:

```tsx
const queryClient = useQueryClient();
```

Then in the `socket.onmessage` handler, after dispatching to listeners, add handling for `task_updated`:

```tsx
socket.onmessage = (event) => {
  try {
    const message = JSON.parse(event.data);

    // Handle task_updated: sync query cache so boards update in real-time
    if (message.type === 'task_updated' && message.task) {
      const task = message.task as Task;
      queryClient.setQueryData(['tasks', task.id], task);
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    }

    const callbacks = listeners.get(message.type);
    if (callbacks) {
      callbacks.forEach((cb) => cb(message));
    }
    const anyCallbacks = listeners.get('*');
    if (anyCallbacks) {
      anyCallbacks.forEach((cb) => cb(message));
    }
  } catch {
    logger.error('Failed to parse WebSocket message');
  }
};
```

**Important:** The `queryClient` reference in `socket.onmessage` is a closure. Since `connect` is a `useCallback` that depends on `listeners`, and `queryClient` is stable (from `useQueryClient`), this should work without stale closure issues. If you see stale queryClient issues, use a ref: `const queryClientRef = useRef(queryClient); queryClientRef.current = queryClient;` and use `queryClientRef.current` in the closure.

### Step 2: Check provider nesting order

Run: `grep -n "WebSocketProvider\|QueryClientProvider" frontend/src/app/layout.tsx 2>/dev/null || grep -rn "WebSocketProvider\|QueryClientProvider" frontend/src/app/ 2>/dev/null | head -10`

If `WebSocketProvider` wraps `QueryClientProvider` (i.e., WS is the outer provider), `useQueryClient()` inside `WebSocketProvider` won't work. In that case, swap the nesting order so `QueryClientProvider` is outer.

### Step 3: Verify TypeScript compiles

Run: `cd frontend && npx tsc --noEmit 2>&1`

Expected: No errors.

### Step 4: Run frontend tests

Run: `cd frontend && npx vitest run 2>&1`

Expected: All existing tests pass (we didn't change test-covered behavior, just added a handler).

### Step 5: Commit

```bash
git add frontend/src/contexts/websocket-context.tsx
git commit -m "feat: sync task query cache from task_updated WS events"
```

---

## Task 8: End-to-end smoke test and verification

### Step 1: Start the app

Run: `cd /home/utility/Projects/ai-kanban && bash start.sh`

(Or however the dev environment is started — check `start.sh` for the correct command.)

### Step 2: Create a task and start a session

1. Open the app in browser
2. Create a new task with a simple title like "List files in current directory"
3. Set project path to a real directory
4. Start a session

### Step 3: Verify stage progression

Observe the stage badge on the task card in the kanban:
- Immediately after start: badge should show **Planning**
- Once Claude starts using tools: badge should change to **In Progress** (within a few seconds)
- After session completes: badge should show **Review**

### Step 4: Verify Live Output Panel

In the task detail:
- Panel should show human-readable lines like `📖 Read: ...`, `⚡ Bash: ...`
- Header should show `● Running Xs` (with seconds ticking up)
- If Claude pauses >8s: header should show `● Waiting...`
- After completion: header should show `Completed`

### Step 5: Run backend tests one final time

Run: `cd backend && cargo test 2>&1`

Expected: All tests pass.

### Step 6: Run frontend tests one final time

Run: `cd frontend && npx vitest run 2>&1`

Expected: All tests pass.

### Step 7: Final commit

```bash
git add -A
git commit -m "feat: live task status with stage progression and heartbeat

- ClaudeEvent enum replaces SessionOutput broadcast
- JSONL parsed to human-readable output (emojis + tool names)
- Task auto-advances: planning → in_progress (first tool) → review (complete)
- Heartbeat every 5s shows running time in Live Panel
- task_updated WS events update kanban board in real-time"
```
