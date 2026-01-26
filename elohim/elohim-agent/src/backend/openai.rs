//! OpenAI-compatible LLM backend.
//!
//! Works with any OpenAI-compatible API including:
//! - vLLM
//! - Ollama
//! - OpenAI API
//! - Azure OpenAI
//! - LocalAI
//! - Together.ai

use async_trait::async_trait;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};

use super::traits::*;

/// OpenAI-compatible backend.
pub struct OpenAiBackend {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    capabilities: ModelCapabilities,
}

impl OpenAiBackend {
    /// Create a new OpenAI-compatible backend.
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
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
            base_url: base_url.into(),
            api_key,
            model: model.into(),
            capabilities: ModelCapabilities {
                context_window: 128_000,
                max_output_tokens: 4096,
                supports_streaming: true,
                supports_json_mode: true,
                supports_function_calling: true,
            },
        }
    }

    /// Create a backend pointing to local vLLM server.
    pub fn vllm(port: u16, model: &str) -> Self {
        Self::new(format!("http://localhost:{}/v1", port), model, None)
    }

    /// Create a backend pointing to Ollama.
    pub fn ollama(model: &str) -> Self {
        Self::new("http://localhost:11434/v1", model, None)
    }

    /// Create a backend for OpenAI API.
    pub fn openai(model: &str, api_key: impl Into<String>) -> Self {
        Self::new("https://api.openai.com/v1", model, Some(api_key.into()))
    }

    /// Set custom capabilities.
    pub fn with_capabilities(mut self, capabilities: ModelCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Build the request URL.
    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// Build authorization header if API key is set.
    fn auth_header(&self) -> Option<String> {
        self.api_key.as_ref().map(|k| format!("Bearer {}", k))
    }
}

/// OpenAI chat completion request body.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormatRequest>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormatRequest {
    #[serde(rename = "type")]
    format_type: String,
}

/// OpenAI chat completion response.
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<UsageResponse>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[async_trait]
impl LlmBackend for OpenAiBackend {
    fn id(&self) -> &str {
        &self.model
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/models", self.base_url);
        let mut request = self.client.get(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header(header::AUTHORIZATION, auth);
        }

        request
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut messages: Vec<ChatMessage> = Vec::new();

        // Add system prompt if present
        if let Some(system) = &request.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add conversation messages
        for msg in &request.messages {
            messages.push(ChatMessage {
                role: match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                }
                .to_string(),
                content: msg.content.clone(),
            });
        }

        let response_format = request.response_format.as_ref().map(|rf| {
            ResponseFormatRequest {
                format_type: match rf.format_type {
                    ResponseFormatType::Json => "json_object",
                    ResponseFormatType::JsonSchema => "json_object",
                    ResponseFormatType::Text => "text",
                }
                .to_string(),
            }
        });

        let chat_request = ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stop: request.stop_sequences.clone(),
            response_format,
            stream: false,
        };

        let mut http_request = self.client.post(self.chat_completions_url());

        if let Some(auth) = self.auth_header() {
            http_request = http_request.header(header::AUTHORIZATION, auth);
        }

        let response = http_request
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited { retry_after_ms: None });
            }

            return Err(LlmError::RequestFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let choice = chat_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ParseError("No choices in response".to_string()))?;

        let content = choice
            .message
            .content
            .unwrap_or_default();

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        };

        let usage = chat_response.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
        }).unwrap_or_default();

        Ok(CompletionResponse {
            content,
            finish_reason,
            usage,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<crate::stream::TokenStream, LlmError> {
        // For now, fall back to non-streaming and wrap the result
        // Full streaming implementation would use SSE
        let response = self.complete(request).await?;
        Ok(crate::stream::TokenStream::from_complete(response))
    }

    fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vllm_creation() {
        let backend = OpenAiBackend::vllm(8000, "llama-3.3-70b");
        assert_eq!(backend.id(), "llama-3.3-70b");
        assert!(backend.capabilities().supports_streaming);
    }

    #[test]
    fn test_ollama_creation() {
        let backend = OpenAiBackend::ollama("llama3.2");
        assert_eq!(backend.id(), "llama3.2");
    }
}
