//! Mock LLM backend for testing.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use super::traits::*;

/// Mock backend for testing.
///
/// Configurable responses and behavior for unit tests.
pub struct MockBackend {
    model_id: String,
    available: AtomicBool,
    capabilities: ModelCapabilities,
    response_content: String,
    call_count: AtomicU32,
}

impl MockBackend {
    /// Create a new mock backend.
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            available: AtomicBool::new(true),
            capabilities: ModelCapabilities::default(),
            response_content: "Mock response".to_string(),
            call_count: AtomicU32::new(0),
        }
    }

    /// Set the response content.
    pub fn with_response(mut self, content: impl Into<String>) -> Self {
        self.response_content = content.into();
        self
    }

    /// Set availability.
    pub fn with_available(self, available: bool) -> Self {
        self.available.store(available, Ordering::SeqCst);
        self
    }

    /// Set capabilities.
    pub fn with_capabilities(mut self, capabilities: ModelCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Get the number of times complete was called.
    pub fn call_count(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Reset the call count.
    pub fn reset_call_count(&self) {
        self.call_count.store(0, Ordering::SeqCst);
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new("mock-model")
    }
}

#[async_trait]
impl LlmBackend for MockBackend {
    fn id(&self) -> &str {
        &self.model_id
    }

    async fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if !self.available.load(Ordering::SeqCst) {
            return Err(LlmError::Unavailable("Mock backend disabled".to_string()));
        }

        // Estimate token counts
        let prompt_tokens: u32 = request
            .messages
            .iter()
            .map(|m| m.content.len() as u32 / 4)
            .sum();

        let completion_tokens = self.response_content.len() as u32 / 4;

        Ok(CompletionResponse {
            content: self.response_content.clone(),
            finish_reason: FinishReason::Stop,
            usage: Usage {
                prompt_tokens,
                completion_tokens,
            },
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<crate::stream::TokenStream, LlmError> {
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

    #[tokio::test]
    async fn test_mock_backend() {
        let backend = MockBackend::new("test-model")
            .with_response("Hello, world!");

        assert!(backend.is_available().await);
        assert_eq!(backend.call_count(), 0);

        let response = backend
            .complete(CompletionRequest::user("Hi"))
            .await
            .unwrap();

        assert_eq!(response.content, "Hello, world!");
        assert_eq!(backend.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_unavailable() {
        let backend = MockBackend::new("test-model").with_available(false);

        assert!(!backend.is_available().await);

        let result = backend.complete(CompletionRequest::user("Hi")).await;
        assert!(result.is_err());
    }
}
