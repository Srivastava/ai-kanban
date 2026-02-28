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
