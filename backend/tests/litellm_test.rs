use ai_kanban_backend::ai::litellm::{image_to_data_url, build_user_message, LitellmClient};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

// ---------------------------------------------------------------------------
// image_to_data_url tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn image_to_data_url_png_returns_data_url() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("test.png");
    // Write a minimal valid-ish PNG header (just a few bytes — enough for reading)
    tokio::fs::write(&file_path, b"\x89PNG\r\n\x1a\n").await.expect("write png");

    let result = image_to_data_url(file_path.to_str().unwrap(), "image/png").await;
    assert!(result.is_some(), "expected Some for valid png file");
    assert!(
        result.unwrap().starts_with("data:image/png;base64,"),
        "expected data URL with image/png mime type"
    );
}

#[tokio::test]
async fn image_to_data_url_jpeg_returns_data_url() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("test.jpg");
    // Write a minimal JPEG-ish bytes
    tokio::fs::write(&file_path, b"\xFF\xD8\xFF\xE0").await.expect("write jpeg");

    let result = image_to_data_url(file_path.to_str().unwrap(), "image/jpeg").await;
    assert!(result.is_some(), "expected Some for valid jpeg file");
    assert!(
        result.unwrap().starts_with("data:image/jpeg;base64,"),
        "expected data URL with image/jpeg mime type"
    );
}

#[tokio::test]
async fn image_to_data_url_nonexistent_returns_none() {
    let result = image_to_data_url("/tmp/nonexistent_file_abc123.png", "image/png").await;
    assert!(result.is_none(), "expected None for nonexistent file");
}

// ---------------------------------------------------------------------------
// build_user_message tests
// ---------------------------------------------------------------------------

#[test]
fn build_user_message_no_images_returns_string_content() {
    let msg = build_user_message("hello", &[]);
    assert_eq!(msg["role"], "user", "role should be user");
    assert_eq!(msg["content"], "hello", "content should be plain string");
    // Ensure content is a string, not an array
    assert!(msg["content"].is_string(), "content should be a string value");
}

#[test]
fn build_user_message_with_images_returns_content_array() {
    let data_url = "data:image/png;base64,abc";
    let msg = build_user_message("hello", &[data_url]);
    assert_eq!(msg["role"], "user", "role should be user");

    let content = &msg["content"];
    assert!(content.is_array(), "content should be an array when images are provided");

    let arr = content.as_array().unwrap();
    assert_eq!(arr.len(), 2, "expected 2 content parts: text + image_url");

    // First part: text
    assert_eq!(arr[0]["type"], "text");
    assert_eq!(arr[0]["text"], "hello");

    // Second part: image_url
    assert_eq!(arr[1]["type"], "image_url");
    assert_eq!(arr[1]["image_url"]["url"], data_url);
}

// ---------------------------------------------------------------------------
// complete_json wiremock tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn complete_json_parses_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "hello world"}}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5}
            })))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let messages = vec![serde_json::json!({"role": "user", "content": "hi"})];
    let result = client.complete_json(messages).await;

    assert!(result.is_ok(), "expected Ok from complete_json, got: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.content, "hello world");
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 5);
}

#[tokio::test]
async fn complete_json_empty_choices_returns_err() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "choices": [],
                "usage": {"prompt_tokens": 0, "completion_tokens": 0}
            })))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let messages = vec![serde_json::json!({"role": "user", "content": "hi"})];
    let result = client.complete_json(messages).await;

    assert!(result.is_err(), "expected Err when choices is empty");
}

#[tokio::test]
async fn complete_json_http_500_returns_err() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500)
            .set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = LitellmClient::new(mock_server.uri(), "test-key", "test-model");
    let messages = vec![serde_json::json!({"role": "user", "content": "hi"})];
    let result = client.complete_json(messages).await;

    assert!(result.is_err(), "expected Err on HTTP 500");
}
