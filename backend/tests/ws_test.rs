use ai_kanban_backend::ws::{ClientMessage, ServerMessage};

#[test]
fn test_client_message_serialize_subscribe_task() {
    let msg = ClientMessage::SubscribeTask {
        task_id: "task-123".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"subscribe_task\""));
    assert!(json.contains("\"task_id\":\"task-123\""));
}

#[test]
fn test_client_message_serialize_subscribe_session() {
    let msg = ClientMessage::SubscribeSession {
        session_id: "session-456".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"subscribe_session\""));
    assert!(json.contains("\"session_id\":\"session-456\""));
}

#[test]
fn test_client_message_serialize_ping() {
    let msg = ClientMessage::Ping;

    let json = serde_json::to_string(&msg).unwrap();
    assert_eq!(json, r#"{"type":"ping"}"#);
}

#[test]
fn test_client_message_deserialize_subscribe_task() {
    let json = r#"{"type":"subscribe_task","task_id":"task-123"}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();

    match msg {
        ClientMessage::SubscribeTask { task_id } => {
            assert_eq!(task_id, "task-123");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_client_message_deserialize_ping() {
    let json = r#"{"type":"ping"}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();

    match msg {
        ClientMessage::Ping => {}
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_server_message_session_output() {
    let msg = ServerMessage::session_output(
        "session-123".to_string(),
        "Claude is thinking...".to_string(),
        false,
    );

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"session_output\""));
    assert!(json.contains("\"session_id\":\"session-123\""));
    assert!(json.contains("\"output\":\"Claude is thinking...\""));
    assert!(json.contains("\"is_error\":false"));
}

#[test]
fn test_server_message_session_status() {
    let msg = ServerMessage::session_status("session-123".to_string(), "running".to_string());

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"session_status\""));
    assert!(json.contains("\"status\":\"running\""));
}

#[test]
fn test_server_message_error() {
    let msg = ServerMessage::error("Something went wrong");

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"error\""));
    assert!(json.contains("\"message\":\"Something went wrong\""));
}

#[test]
fn test_server_message_subscribed() {
    let msg = ServerMessage::subscribed("task-123");

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"subscribed\""));
    assert!(json.contains("\"topic\":\"task-123\""));
}

#[test]
fn test_server_message_pong() {
    let msg = ServerMessage::pong();

    let json = serde_json::to_string(&msg).unwrap();
    assert_eq!(json, r#"{"type":"pong"}"#);
}

#[test]
fn test_server_message_task_updated() {
    let task = serde_json::json!({
        "id": "task-123",
        "title": "Test Task"
    });

    let msg = ServerMessage::TaskUpdated { task };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"task_updated\""));
    assert!(json.contains("\"task\""));
}

#[test]
fn test_client_message_deserialize_subscribe_session() {
    let json = r#"{"type":"subscribe_session","session_id":"session-456"}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();

    match msg {
        ClientMessage::SubscribeSession { session_id } => {
            assert_eq!(session_id, "session-456");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_server_message_session_output_with_error() {
    let msg = ServerMessage::session_output(
        "session-error".to_string(),
        "Error occurred".to_string(),
        true,
    );

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"is_error\":true"));
}

#[test]
fn test_server_message_session_status_completed() {
    let msg = ServerMessage::session_status("session-done".to_string(), "completed".to_string());

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"status\":\"completed\""));
}
