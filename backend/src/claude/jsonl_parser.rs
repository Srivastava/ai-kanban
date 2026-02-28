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

/// Extract the final result text from a Claude result line.
/// Returns Some(text) for {"type":"result","subtype":"success","result":"..."}
pub fn extract_result_text(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "result" {
        return None;
    }
    value.get("result")?.as_str().map(|s| s.to_string())
}
