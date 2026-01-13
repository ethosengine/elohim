//! Core traits for LLM backends.
//!
//! This module defines the `LlmBackend` trait - the primary abstraction
//! over different LLM inference engines.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Error types for LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Backend is not available
    #[error("Backend unavailable: {0}")]
    Unavailable(String),

    /// Request failed
    #[error("Request failed: {0}")]
    RequestFailed(String),

    /// Rate limited by the backend
    #[error("Rate limited, retry after {retry_after_ms:?}ms")]
    RateLimited { retry_after_ms: Option<u64> },

    /// Input exceeded context length
    #[error("Context length exceeded: {max} tokens, got {actual}")]
    ContextLengthExceeded { max: u32, actual: u32 },

    /// Content was filtered
    #[error("Content filtered: {reason}")]
    ContentFiltered { reason: String },

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Parsing error
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Core trait for LLM backends.
///
/// This trait abstracts over different inference engines (vLLM, OpenAI, llama.cpp)
/// providing a consistent interface for the agent system.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Get the backend identifier (e.g., model name).
    fn id(&self) -> &str;

    /// Check if the backend is currently available.
    async fn is_available(&self) -> bool;

    /// Generate a completion (non-streaming).
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Generate a streaming completion.
    ///
    /// Returns a stream of response chunks.
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<crate::stream::TokenStream, LlmError>;

    /// Get the capabilities of this backend.
    fn capabilities(&self) -> &ModelCapabilities;
}

/// Request for LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct CompletionRequest {
    /// System prompt (optional)
    pub system_prompt: Option<String>,
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0-2.0, default 1.0)
    pub temperature: Option<f32>,
    /// Sequences that stop generation
    pub stop_sequences: Vec<String>,
    /// Request structured output format
    pub response_format: Option<ResponseFormat>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            system_prompt: None,
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            stop_sequences: Vec::new(),
            response_format: None,
        }
    }
}

impl CompletionRequest {
    /// Create a new request with a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            messages: vec![Message::user(content)],
            ..Default::default()
        }
    }

    /// Add a system prompt.
    pub fn with_system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a message.
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Request JSON output.
    pub fn with_json_output(mut self) -> Self {
        self.response_format = Some(ResponseFormat {
            format_type: ResponseFormatType::Json,
            schema: None,
        });
        self
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Response from LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct CompletionResponse {
    /// Generated content
    pub content: String,
    /// Why generation stopped
    pub finish_reason: FinishReason,
    /// Token usage
    pub usage: Usage,
}

/// Why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop (end of response or stop sequence)
    Stop,
    /// Hit max tokens limit
    Length,
    /// Content was filtered
    ContentFilter,
}

/// Token usage information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
}

impl Usage {
    /// Get total tokens.
    pub fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Format for structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ResponseFormat {
    /// Type of format
    pub format_type: ResponseFormatType,
    /// JSON schema (for JsonSchema type)
    pub schema: Option<serde_json::Value>,
}

/// Type of response format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormatType {
    /// Plain text
    Text,
    /// JSON object
    Json,
    /// JSON conforming to schema
    JsonSchema,
}

/// Capabilities of a model/backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ModelCapabilities {
    /// Maximum context window size
    pub context_window: u32,
    /// Maximum output tokens
    pub max_output_tokens: u32,
    /// Whether streaming is supported
    pub supports_streaming: bool,
    /// Whether JSON mode is supported
    pub supports_json_mode: bool,
    /// Whether function/tool calling is supported
    pub supports_function_calling: bool,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            context_window: 4096,
            max_output_tokens: 1024,
            supports_streaming: false,
            supports_json_mode: false,
            supports_function_calling: false,
        }
    }
}
