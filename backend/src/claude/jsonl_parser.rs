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

/// Extract Claude's internal session_id from the init system event.
/// Returns Some(uuid) for {"type":"system","subtype":"init","session_id":"<uuid>"}
pub fn extract_claude_session_id(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "system" {
        return None;
    }
    if value.get("subtype")?.as_str()? != "init" {
        return None;
    }
    value.get("session_id")?.as_str().map(|s| s.to_string())
}

/// Extract the final result text from a Claude result line.
/// Returns Some(text) for {"type":"result","subtype":"success","result":"..."}
pub fn extract_result_text(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "result" {
        return None;
    }
    value.get("result")?.as_str().map(|s| s.to_string())
}

/// Truncate a string to at most `max_bytes` bytes, ensuring we don't cut in the middle
/// of a multi-byte UTF-8 character. Returns a `&str` slice.
fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &s[..boundary]
}

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
                        format!("{}...", truncate_to_char_boundary(text, 120))
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
            let preview = truncate_to_char_boundary(cmd, 80);
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
                let preview = truncate_to_char_boundary(s, 80);
                return Some(preview.to_string());
            }
        }
    }
    None
}
