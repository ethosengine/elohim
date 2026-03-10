//! Anthropic Messages API backend.
//!
//! Implements the LlmBackend trait for Anthropic's Claude models.
//! Used for BYOK (Bring Your Own Key) scenarios where users provide
//! their own API key through the frontend.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};

use super::traits::*;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Anthropic Messages API backend.
///
/// Supports Claude Sonnet, Opus, and Haiku models.
/// Primarily used for BYOK where user-provided API keys
/// are forwarded from the Angular frontend via doorway.
pub struct AnthropicBackend {
    client: Client,
    api_key: String,
    model: String,
    capabilities: ModelCapabilities,
}

impl AnthropicBackend {
    /// Create a new Anthropic backend with explicit model.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.into(),
            model: model.into(),
            capabilities: ModelCapabilities {
                context_window: 200_000,
                max_output_tokens: 8192,
                supports_streaming: true,
                supports_json_mode: true,
                supports_function_calling: true,
            },
        }
    }

    /// Create a backend for Claude Sonnet.
    pub fn claude_sonnet(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "claude-sonnet-4-20250514")
    }

    /// Create a backend for Claude Opus.
    pub fn claude_opus(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "claude-opus-4-20250514")
    }

    /// Create a backend for Claude Haiku.
    pub fn claude_haiku(api_key: impl Into<String>) -> Self {
        Self::new(api_key, "claude-haiku-4-20250514")
    }

    /// Set custom capabilities.
    pub fn with_capabilities(mut self, capabilities: ModelCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Build API messages from a CompletionRequest.
    /// System prompt is handled separately (top-level field in Anthropic API).
    fn build_messages(&self, request: &CompletionRequest) -> Vec<ApiMessage> {
        request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| ApiMessage {
                role: match m.role {
                    MessageRole::User | MessageRole::System => "user",
                    MessageRole::Assistant => "assistant",
                }
                .to_string(),
                content: m.content.clone(),
            })
            .collect()
    }

    /// Process SSE byte stream into TokenStream chunks.
    async fn process_sse_stream(
        mut bytes_stream: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>>
            + Unpin
            + Send,
        sender: crate::stream::TokenStreamSender,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut buffer = String::new();
        let mut finish_reason = FinishReason::Stop;

        'outer: while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim_end().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break 'outer;
                    }

                    let value: serde_json::Value = match serde_json::from_str(data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let event_type = value
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");

                    match event_type {
                        "content_block_delta" => {
                            if let Ok(delta) =
                                serde_json::from_value::<SseContentBlockDelta>(value)
                            {
                                if delta.delta.delta_type == "text_delta"
                                    && !delta.delta.text.is_empty()
                                {
                                    let _ = sender.send(delta.delta.text).await;
                                }
                            }
                        }
                        "message_delta" => {
                            if let Ok(msg_delta) =
                                serde_json::from_value::<SseMessageDelta>(value)
                            {
                                finish_reason = match msg_delta.delta.stop_reason.as_deref() {
                                    Some("max_tokens") => FinishReason::Length,
                                    _ => FinishReason::Stop,
                                };
                            }
                        }
                        "message_stop" => {
                            break 'outer;
                        }
                        _ => {
                            // Ignore: message_start, content_block_start,
                            // content_block_stop, ping
                        }
                    }
                }
            }
        }

        let _ = sender.finish("", finish_reason).await;
        Ok(())
    }
}

// ============================================================================
// Wire types for Anthropic Messages API
// ============================================================================

#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    usage: ApiUsage,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// SSE event types for streaming

#[derive(Debug, Deserialize)]
struct SseContentBlockDelta {
    delta: TextDelta,
}

#[derive(Debug, Deserialize)]
struct TextDelta {
    #[serde(rename = "type")]
    delta_type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct SseMessageDelta {
    delta: MessageDeltaBody,
    #[allow(dead_code)] // Needed for serde, may be used for cost tracking later
    usage: Option<DeltaUsage>,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaBody {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Wire type for serde deserialization
struct DeltaUsage {
    output_tokens: u32,
}

// ============================================================================
// LlmBackend implementation
// ============================================================================

#[async_trait]
impl LlmBackend for AnthropicBackend {
    fn id(&self) -> &str {
        &self.model
    }

    async fn is_available(&self) -> bool {
        // Anthropic doesn't have a lightweight health endpoint.
        // A non-empty API key is our best availability signal.
        !self.api_key.is_empty()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let messages = self.build_messages(&request);

        let body = MessagesRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(2048),
            system: request.system_prompt.clone(),
            messages,
            temperature: request.temperature,
            stop_sequences: request.stop_sequences.clone(),
            stream: false,
        };

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited {
                    retry_after_ms: None,
                });
            }

            return Err(LlmError::RequestFailed(format!(
                "HTTP {}: {}",
                status, err_body
            )));
        }

        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let content = data
            .content
            .into_iter()
            .filter_map(|b| {
                if b.block_type == "text" {
                    b.text
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        let finish_reason = match data.stop_reason.as_deref() {
            Some("max_tokens") => FinishReason::Length,
            _ => FinishReason::Stop,
        };

        Ok(CompletionResponse {
            content,
            finish_reason,
            usage: Usage {
                prompt_tokens: data.usage.input_tokens,
                completion_tokens: data.usage.output_tokens,
            },
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<crate::stream::TokenStream, LlmError> {
        let messages = self.build_messages(&request);

        let body = MessagesRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(2048),
            system: request.system_prompt.clone(),
            messages,
            temperature: request.temperature,
            stop_sequences: request.stop_sequences.clone(),
            stream: true,
        };

        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited {
                    retry_after_ms: None,
                });
            }

            return Err(LlmError::RequestFailed(format!(
                "HTTP {}: {}",
                status, err_body
            )));
        }

        let (sender, stream) = crate::stream::TokenStream::channel(64);

        let bytes_stream = response.bytes_stream();
        tokio::spawn(async move {
            if let Err(e) = Self::process_sse_stream(bytes_stream, sender).await {
                tracing::warn!("Anthropic SSE stream error: {}", e);
            }
        });

        Ok(stream)
    }

    fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sonnet_creation() {
        let backend = AnthropicBackend::claude_sonnet("test-key");
        assert_eq!(backend.id(), "claude-sonnet-4-20250514");
        assert!(backend.capabilities().supports_streaming);
        assert_eq!(backend.capabilities().context_window, 200_000);
    }

    #[test]
    fn test_opus_creation() {
        let backend = AnthropicBackend::claude_opus("test-key");
        assert_eq!(backend.id(), "claude-opus-4-20250514");
    }

    #[test]
    fn test_haiku_creation() {
        let backend = AnthropicBackend::claude_haiku("test-key");
        assert_eq!(backend.id(), "claude-haiku-4-20250514");
    }

    #[test]
    fn test_custom_model() {
        let backend = AnthropicBackend::new("key", "claude-custom-model");
        assert_eq!(backend.id(), "claude-custom-model");
    }

    #[tokio::test]
    async fn test_available_with_key() {
        let backend = AnthropicBackend::new("valid-key", "model");
        assert!(backend.is_available().await);
    }

    #[tokio::test]
    async fn test_unavailable_without_key() {
        let backend = AnthropicBackend::new("", "model");
        assert!(!backend.is_available().await);
    }

    #[test]
    fn test_request_serialization() {
        let request = MessagesRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            system: Some("You are helpful.".to_string()),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.7),
            stop_sequences: vec![],
            stream: false,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["system"], "You are helpful.");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["stream"], false);
        // stop_sequences should be skipped (empty vec)
        assert!(json.get("stop_sequences").is_none());
    }

    #[test]
    fn test_request_without_optional_fields() {
        let request = MessagesRequest {
            model: "model".to_string(),
            max_tokens: 1024,
            system: None,
            messages: vec![],
            temperature: None,
            stop_sequences: vec![],
            stream: false,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("system").is_none());
        assert!(json.get("temperature").is_none());
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{
            "content": [{"type": "text", "text": "Hello, world!"}],
            "usage": {"input_tokens": 10, "output_tokens": 5},
            "stop_reason": "end_turn"
        }"#;

        let response: MessagesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].text.as_deref(), Some("Hello, world!"));
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
        assert_eq!(response.stop_reason.as_deref(), Some("end_turn"));
    }

    #[test]
    fn test_sse_content_block_delta_deserialization() {
        let json = r#"{
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "Hello"}
        }"#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let delta: SseContentBlockDelta = serde_json::from_value(value).unwrap();
        assert_eq!(delta.delta.text, "Hello");
        assert_eq!(delta.delta.delta_type, "text_delta");
    }

    #[test]
    fn test_sse_message_delta_deserialization() {
        let json = r#"{
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 42}
        }"#;

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let delta: SseMessageDelta = serde_json::from_value(value).unwrap();
        assert_eq!(delta.delta.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(delta.usage.unwrap().output_tokens, 42);
    }

    #[test]
    fn test_build_messages_filters_system() {
        let backend = AnthropicBackend::new("key", "model");
        let request = CompletionRequest {
            system_prompt: Some("System prompt".to_string()),
            messages: vec![
                Message::system("Should be filtered"),
                Message::user("Hello"),
                Message::assistant("Hi there"),
            ],
            ..Default::default()
        };

        let messages = backend.build_messages(&request);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[test]
    fn test_with_capabilities() {
        let caps = ModelCapabilities {
            context_window: 100_000,
            max_output_tokens: 4096,
            supports_streaming: false,
            supports_json_mode: false,
            supports_function_calling: false,
        };

        let backend = AnthropicBackend::new("key", "model").with_capabilities(caps);
        assert_eq!(backend.capabilities().context_window, 100_000);
        assert!(!backend.capabilities().supports_streaming);
    }
}
