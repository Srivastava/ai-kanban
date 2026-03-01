use ai_kanban_backend::claude::jsonl_parser::parse_for_display;
use ai_kanban_backend::claude::jsonl_parser::parse_jsonl_line;

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
    assert_eq!(event.file_ext, None);
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
    let line = r#"{"type":"assistant","message":{"content":[]}}"#;
    let event = parse_jsonl_line(line);
    assert!(event.is_none());
}

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
