use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct LitellmClient {
    client: Client,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: i64,
    completion_tokens: i64,
}

#[derive(Debug)]
pub struct CompletionResult {
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub latency_ms: u64,       // wall-clock ms for the HTTP round-trip
    pub tokens_per_sec: f64,   // output_tokens / (latency_ms / 1000)
}

/// Encode an image file as a base64 data URL for the LiteLLM vision API.
/// Returns `None` if the file cannot be read (non-fatal — caller skips that image).
/// Uses the provided `mime_type` directly rather than inferring from the file extension.
pub async fn image_to_data_url(path: &str, mime_type: &str) -> Option<String> {
    use base64::Engine as _;
    let data = tokio::fs::read(path).await.ok()?;
    Some(format!(
        "data:{};base64,{}",
        mime_type,
        base64::engine::general_purpose::STANDARD.encode(&data)
    ))
}

impl LitellmClient {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Create a client from environment variables with sane defaults.
    /// LITELLM_BASE_URL (default: http://192.168.4.118:14000), LITELLM_API_KEY, LITELLM_MODEL
    pub fn from_env() -> Self {
        let base_url = std::env::var("LITELLM_BASE_URL")
            .unwrap_or_else(|_| "http://192.168.4.118:14000".to_string());
        let api_key = std::env::var("LITELLM_API_KEY")
            .unwrap_or_else(|_| "litellm".to_string());
        let model = std::env::var("LITELLM_MODEL")
            .unwrap_or_else(|_| "smart-router".to_string());
        Self::new(base_url, api_key, model)
    }

    pub async fn complete(&self, messages: Vec<ChatMessage>) -> Result<CompletionResult> {
        let json_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        self.complete_json(json_messages).await
    }

    /// Send a completion request with pre-built JSON message values.
    /// Use this when messages contain multimodal content (images).
    pub async fn complete_json(&self, messages: Vec<serde_json::Value>) -> Result<CompletionResult> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });

        debug!(
            model = %self.model,
            url = %url,
            messages = messages.len(),
            "Sending LiteLLM request"
        );

        let t_start = std::time::Instant::now();
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            warn!(status = %status, body = %text, "LiteLLM request failed");
            return Err(anyhow::anyhow!("LiteLLM error {}: {}", status, text));
        }

        let resp: CompletionResponse = response.json().await?;
        let latency_ms = t_start.elapsed().as_millis() as u64;

        let first = resp.choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("LiteLLM returned empty choices"))?;
        let content = first.message.content
            .ok_or_else(|| anyhow::anyhow!("LiteLLM returned null content"))?;

        let (input_tokens, output_tokens) = resp.usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        let tokens_per_sec = if latency_ms > 0 {
            output_tokens as f64 / (latency_ms as f64 / 1000.0)
        } else {
            0.0
        };

        info!(
            model = %self.model,
            input_tokens = input_tokens,
            output_tokens = output_tokens,
            latency_ms = latency_ms,
            tokens_per_sec = tokens_per_sec,
            content_len = content.len(),
            "LiteLLM completion received"
        );

        Ok(CompletionResult { content, input_tokens, output_tokens, latency_ms, tokens_per_sec })
    }
}
