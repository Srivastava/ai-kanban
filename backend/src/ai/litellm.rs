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
    content: String,
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
    /// LITELLM_BASE_URL, LITELLM_API_KEY, LITELLM_MODEL
    pub fn from_env() -> Self {
        let base_url = std::env::var("LITELLM_BASE_URL")
            .unwrap_or_else(|_| "http://192.168.4.118:14000".to_string());
        let api_key = std::env::var("LITELLM_API_KEY")
            .unwrap_or_else(|_| "litellm".to_string());
        let model = std::env::var("LITELLM_MODEL")
            .unwrap_or_else(|_| "qwen2.5:latest".to_string());
        Self::new(base_url, api_key, model)
    }

    pub async fn complete(&self, messages: Vec<ChatMessage>) -> Result<CompletionResult> {
        let url = format!("{}/chat/completions", self.base_url);

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

        let content = resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        let (input_tokens, output_tokens) = resp.usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        info!(
            model = %self.model,
            input_tokens = input_tokens,
            output_tokens = output_tokens,
            content_len = content.len(),
            "LiteLLM completion received"
        );

        Ok(CompletionResult { content, input_tokens, output_tokens })
    }
}
