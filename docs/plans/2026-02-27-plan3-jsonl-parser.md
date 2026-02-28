# JSONL Parser + ClaudeManager Integration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Parse Claude's streaming JSONL output to extract per-event token usage and store in `token_events` and `session_metrics` tables.

**Architecture:** Add a `jsonl_parser` module that tries to parse each stdout line as JSON and extracts token counts, tool names, and file extensions. Update `ClaudeManager` to: (1) add `--output-format stream-json` to the command, (2) pipe stdout through the parser and store token events, (3) snapshot project metrics at session start. Wire the new repositories into `AppState`.

**Tech Stack:** Rust, serde_json, std::path::Path, tokio spawn_blocking

---

## Context

Key files:
- `backend/src/claude/manager.rs` — spawns Claude, reads stdout/stderr line by line
- `backend/src/api/mod.rs` — AppState and individual API states
- `backend/src/main.rs` — wires repositories into AppState

**Claude stream-json output format.** When `--output-format stream-json` is added, each stdout line is a JSON object. Key event types:

```json
// assistant turn — contains usage + tool calls in content array
{"type":"assistant","message":{"id":"msg_...","content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/main.rs"}}],"model":"claude-sonnet-4-6","usage":{"input_tokens":1234,"output_tokens":56}}}

// result — final summary
{"type":"result","subtype":"success","usage":{"input_tokens":5678,"output_tokens":890},"session_id":"..."}

// system — init event, ignore for tokens
{"type":"system","subtype":"init","model":"claude-sonnet-4-6","sessionId":"..."}
```

---

## Task 1: JSONL Parser Module

**Files:**
- Create: `backend/src/claude/jsonl_parser.rs`

**Step 1: Write the test file**

Create `backend/tests/jsonl_parser_test.rs`:

```rust
use ai_kanban_backend::claude::jsonl_parser::{parse_jsonl_line, ParsedTokenEvent};

#[test]
fn test_parse_assistant_with_tool_use() {
    let line = r#"{"type":"assistant","message":{"id":"msg_1","content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/main.rs"}}],"model":"claude-sonnet-4-6","stop_reason":"tool_use","usage":{"input_tokens":1234,"output_tokens":56}}}"#;

    let event = parse_jsonl_line(line).unwrap();
    assert_eq!(event.event_type, "assistant");
    assert_eq!(event.tool_name, Some("Read".to_string()));
    assert_eq!(event.file_ext, Some(".rs".to_string()));
    assert_eq!(event.input_tokens, 1234);
    assert_eq!(event.output_tokens, 56);
    assert_eq!(event.model, Some("claude-sonnet-4-6".to_string()));
}

#[test]
fn test_parse_result_event() {
    let line = r#"{"type":"result","subtype":"success","usage":{"input_tokens":5678,"output_tokens":890}}"#;

    let event = parse_jsonl_line(line).unwrap();
    assert_eq!(event.event_type, "result");
    assert_eq!(event.tool_name, None);
    assert_eq!(event.input_tokens, 5678);
    assert_eq!(event.output_tokens, 890);
}

#[test]
fn test_parse_system_event_ignored() {
    let line = r#"{"type":"system","subtype":"init","model":"claude-sonnet-4-6"}"#;
    // system events have no token data, so we return None
    let event = parse_jsonl_line(line);
    assert!(event.is_none());
}

#[test]
fn test_parse_plain_text_returns_none() {
    let line = "This is plain text output from Claude";
    let event = parse_jsonl_line(line);
    assert!(event.is_none());
}

#[test]
fn test_parse_bash_tool_no_ext() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"ls -la"}}],"usage":{"input_tokens":100,"output_tokens":10}}}"#;

    let event = parse_jsonl_line(line).unwrap();
    assert_eq!(event.tool_name, Some("Bash".to_string()));
    assert_eq!(event.file_ext, None); // Bash has no file path
}

#[test]
fn test_parse_write_tool_ts_ext() {
    let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"frontend/src/app/page.tsx"}}],"usage":{"input_tokens":500,"output_tokens":200}}}"#;

    let event = parse_jsonl_line(line).unwrap();
    assert_eq!(event.tool_name, Some("Write".to_string()));
    assert_eq!(event.file_ext, Some(".tsx".to_string()));
}

#[test]
fn test_parse_missing_usage_returns_none() {
    // assistant message with no usage block
    let line = r#"{"type":"assistant","message":{"content":[]}}"#;
    let event = parse_jsonl_line(line);
    assert!(event.is_none());
}
```

**Step 2: Run tests to verify they fail**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test jsonl_parser 2>&1 | tail -5
```

Expected: compile error — `jsonl_parser` module not found.

**Step 3: Implement the parser**

Create `backend/src/claude/jsonl_parser.rs`:

```rust
use std::path::Path;

/// Parsed token data from a single JSONL line
#[derive(Debug, Clone)]
pub struct ParsedTokenEvent {
    pub event_type: String,
    pub tool_name: Option<String>,
    pub file_ext: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: Option<String>,
}

/// Try to parse a JSONL line from Claude's stream-json output.
/// Returns None if the line is not JSON, is a system event, or has no token data.
pub fn parse_jsonl_line(line: &str) -> Option<ParsedTokenEvent> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = value.get("type")?.as_str()?.to_string();

    match event_type.as_str() {
        "assistant" => parse_assistant_event(&value),
        "result" => parse_result_event(&value),
        _ => None, // "system", "tool" etc — no token data we want
    }
}

fn parse_assistant_event(value: &serde_json::Value) -> Option<ParsedTokenEvent> {
    let message = value.get("message")?;
    let usage = message.get("usage")?;

    let input_tokens = usage.get("input_tokens")?.as_i64().unwrap_or(0);
    let output_tokens = usage.get("output_tokens")?.as_i64().unwrap_or(0);

    // Only record events that actually have tokens
    if input_tokens == 0 && output_tokens == 0 {
        return None;
    }

    let model = message
        .get("model")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());

    let (tool_name, file_ext) = extract_tool_info(message);

    Some(ParsedTokenEvent {
        event_type: "assistant".to_string(),
        tool_name,
        file_ext,
        input_tokens,
        output_tokens,
        model,
    })
}

fn parse_result_event(value: &serde_json::Value) -> Option<ParsedTokenEvent> {
    let usage = value.get("usage")?;
    let input_tokens = usage.get("input_tokens")?.as_i64().unwrap_or(0);
    let output_tokens = usage.get("output_tokens")?.as_i64().unwrap_or(0);

    if input_tokens == 0 && output_tokens == 0 {
        return None;
    }

    Some(ParsedTokenEvent {
        event_type: "result".to_string(),
        tool_name: None,
        file_ext: None,
        input_tokens,
        output_tokens,
        model: None,
    })
}

/// Extract tool name and file extension from message content array
fn extract_tool_info(message: &serde_json::Value) -> (Option<String>, Option<String>) {
    let content = match message.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return (None, None),
    };

    for item in content {
        if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
            let name = item
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());

            let file_ext = extract_file_ext_from_input(item.get("input"));

            return (name, file_ext);
        }
    }

    (None, None)
}

/// Extract file extension from a tool's input object.
/// Checks common path field names: file_path, path, notebook_path.
fn extract_file_ext_from_input(input: Option<&serde_json::Value>) -> Option<String> {
    let input = input?;

    let path_str = input
        .get("file_path")
        .or_else(|| input.get("path"))
        .or_else(|| input.get("notebook_path"))
        .and_then(|p| p.as_str())?;

    Path::new(path_str)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
}
```

**Step 4: Register in claude/mod.rs**

Open `backend/src/claude/mod.rs`. Add:

```rust
pub mod jsonl_parser;
```

**Step 5: Run tests to verify they pass**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test jsonl_parser 2>&1
```

Expected: 7 tests pass.

**Step 6: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add backend/src/claude/jsonl_parser.rs \
        backend/src/claude/mod.rs \
        backend/tests/jsonl_parser_test.rs
git commit -m "feat(claude): add JSONL parser for token event extraction

- parse_jsonl_line() handles 'assistant' and 'result' event types
- Extracts tool_name, file_ext from tool_use content blocks
- Returns None for plain text, system events, and zero-token lines
- 7 unit tests covering all edge cases"
```

---

## Task 2: Wire Repositories into AppState

**Files:**
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/main.rs`

**Step 1: Add repositories to AppState**

Open `backend/src/api/mod.rs`. Add `TokenEventRepository` and `SessionMetricsRepository` to the imports and `AppState`:

```rust
use crate::db::{CommentRepository, LogRepository, SessionRepository, TaskRepository,
                TokenEventRepository, SessionMetricsRepository};

#[derive(Clone)]
pub struct AppState {
    pub tasks: TaskRepository,
    pub logs: LogRepository,
    pub sessions: SessionRepository,
    pub comments: CommentRepository,
    pub queue: Option<Arc<SessionQueue>>,
    pub token_events: TokenEventRepository,       // NEW
    pub session_metrics: SessionMetricsRepository, // NEW
}

impl AppState {
    pub fn new(
        tasks: TaskRepository,
        logs: LogRepository,
        sessions: SessionRepository,
        comments: CommentRepository,
        token_events: TokenEventRepository,
        session_metrics: SessionMetricsRepository,
    ) -> Self {
        Self {
            tasks,
            logs,
            sessions,
            comments,
            queue: None,
            token_events,
            session_metrics,
        }
    }
    // with_queue stays the same
}
```

Also add a new state type for analytics:

```rust
#[derive(Clone)]
pub struct AnalyticsApiState {
    pub token_events: TokenEventRepository,
    pub session_metrics: SessionMetricsRepository,
}

impl From<AppState> for AnalyticsApiState {
    fn from(state: AppState) -> Self {
        AnalyticsApiState {
            token_events: state.token_events,
            session_metrics: state.session_metrics,
        }
    }
}
```

**Step 2: Update main.rs**

Open `backend/src/main.rs`. Add the new repositories:

```rust
use ai_kanban_backend::db::{
    create_pool, CommentRepository, LogRepository, SessionRepository, TaskRepository,
    TokenEventRepository, SessionMetricsRepository,
};

// In main():
let token_event_repo = TokenEventRepository::new(pool.clone());
let session_metrics_repo = SessionMetricsRepository::new(pool.clone());

let state = AppState::new(
    task_repo,
    log_repo,
    session_repo,
    comment_repo,
    token_event_repo,
    session_metrics_repo,
).with_queue(queue);
```

**Step 3: Verify compilation**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 3: Wire Parser into ClaudeManager

**Files:**
- Modify: `backend/src/claude/manager.rs`

**Step 1: Update ClaudeManager to accept repositories**

Open `backend/src/claude/manager.rs`. Add the new dependencies at the top:

```rust
use crate::claude::jsonl_parser::parse_jsonl_line;
use crate::db::{SessionMetricsRepository, TokenEventRepository};
use crate::models::CreateTokenEvent;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc as StdArc;
```

Update the `ClaudeManager` struct:

```rust
pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,
    output_tx: broadcast::Sender<SessionOutput>,
    session_repo: SessionRepository,
    token_event_repo: TokenEventRepository,   // NEW
    session_metrics_repo: SessionMetricsRepository, // NEW
}

impl ClaudeManager {
    pub fn new(
        session_repo: SessionRepository,
        token_event_repo: TokenEventRepository,
        session_metrics_repo: SessionMetricsRepository,
    ) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            session_repo,
            token_event_repo,
            session_metrics_repo,
        }
    }
```

**Step 2: Add `--output-format stream-json` to the command**

In `start_session`, update the `Command::new("claude")` block:

```rust
let mut child = Command::new("claude")
    .arg("--print")
    .arg("--output-format").arg("stream-json")  // ADD THIS LINE
    .arg(&prompt)
    .current_dir(&task.project_path)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| anyhow!("Failed to spawn Claude: {}", e))?;
```

**Step 3: Snapshot project metrics at session start**

After spawning, before the spawn_blocking calls, add:

```rust
// Snapshot project metrics at session start
let project_metrics = count_project_files(&task.project_path);
let metrics_repo = self.session_metrics_repo.clone();
let session_id_metrics = session.id.clone();
tokio::spawn(async move {
    let _ = metrics_repo
        .upsert(&session_id_metrics, project_metrics.0, project_metrics.1)
        .await;
});
```

Add the helper function at the bottom of the file (outside impl):

```rust
/// Count files and lines of code in a project directory
fn count_project_files(project_path: &str) -> (i64, i64) {
    use std::fs;
    let mut file_count: i64 = 0;
    let mut loc: i64 = 0;

    fn visit_dir(path: &std::path::Path, file_count: &mut i64, loc: &mut i64) {
        let Ok(entries) = fs::read_dir(path) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            // Skip hidden dirs and node_modules
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
```

**Step 4: Parse stdout lines and store token events**

Replace the stdout spawn_blocking with:

```rust
let session_id = session.id.clone();
let output_tx = self.output_tx.clone();
let token_repo = self.token_event_repo.clone();
let task_id_clone = task.id.clone();
let session_id_for_metrics = session.id.clone();
let metrics_repo = self.session_metrics_repo.clone();

tokio::task::spawn_blocking(move || {
    let reader = BufReader::new(stdout);
    let mut sequence_no: i64 = 0;

    for line in reader.lines() {
        let Ok(text) = line else { continue };

        // Try to parse as JSONL token event
        if let Some(parsed) = parse_jsonl_line(&text) {
            let event = CreateTokenEvent {
                session_id: session_id.clone(),
                task_id: task_id_clone.clone(),
                event_type: parsed.event_type,
                tool_name: parsed.tool_name.clone(),
                file_ext: parsed.file_ext,
                input_tokens: parsed.input_tokens,
                output_tokens: parsed.output_tokens,
                model: parsed.model,
                sequence_no: Some(sequence_no),
            };

            // Track lines_written for Write/Edit tools (heuristic: output_tokens / 4)
            if matches!(parsed.tool_name.as_deref(), Some("Write") | Some("Edit")) {
                let approx_lines = (parsed.output_tokens / 4).max(1);
                let m = metrics_repo.clone();
                let sid = session_id_for_metrics.clone();
                tokio::runtime::Handle::current().spawn(async move {
                    let _ = m.add_lines_written(&sid, approx_lines).await;
                });
            }

            let repo = token_repo.clone();
            tokio::runtime::Handle::current().spawn(async move {
                let _ = repo.create(event).await;
            });
        }

        debug!(session_id = %session_id, "stdout: {}", text);
        let _ = output_tx.send(SessionOutput {
            session_id: session_id.clone(),
            line: text,
            is_error: false,
        });

        sequence_no += 1;
    }
});
```

**Step 5: Update main.rs — pass new repos to ClaudeManager**

In `backend/src/main.rs`, update the ClaudeManager construction:

```rust
let claude_manager = Arc::new(ClaudeManager::new(
    session_repo.clone(),
    token_event_repo.clone(),
    session_metrics_repo.clone(),
));
```

**Step 6: Verify compilation**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

**Step 7: Run all backend tests**

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test 2>&1 | tail -20
```

Expected: all existing tests pass plus the new jsonl_parser and token_events tests.

**Step 8: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add backend/src/claude/manager.rs \
        backend/src/api/mod.rs \
        backend/src/main.rs
git commit -m "feat(claude): wire JSONL parser and token events into ClaudeManager

- Add --output-format stream-json flag to Claude command
- Parse each stdout line and store token_event rows
- Snapshot project file count and LOC at session start
- Approximate lines_written from Write/Edit tool output tokens
- TokenEventRepository and SessionMetricsRepository wired into AppState"
```
