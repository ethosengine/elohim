//! ElohimAgentService - main entry point for agent invocation.
//!
//! This service orchestrates LLM backends with constitutional reasoning.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::audit::{AuditEntry, AuditLog};
use crate::backend::traits::LlmBackend;
use crate::capability::{CapabilityRegistry, ElohimCapability};
use crate::request::ElohimRequest;
use crate::response::{ConstitutionalReasoning, ElohimResponse, ResponsePayload, ResponseStatus};
use crate::stream::TokenStream;
use crate::types::ComputationCost;
use constitution::{ConstitutionalStack, PromptAssembler, StackContext};

/// Error types for the service.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// Service not initialized
    #[error("Service not initialized - call initialize() first")]
    NotInitialized,

    /// No backend available
    #[error("No LLM backend available")]
    NoBackendAvailable,

    /// Backend error
    #[error("Backend error: {0}")]
    BackendError(#[from] crate::backend::traits::LlmError),

    /// Constitutional error
    #[error("Constitutional error: {0}")]
    ConstitutionalError(#[from] constitution::stack::ConstitutionError),

    /// Capability not registered
    #[error("Capability not available: {0:?}")]
    CapabilityNotAvailable(ElohimCapability),

    /// Request validation error
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Configuration for the ElohimAgentService.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Agent ID for this service
    pub agent_id: String,
    /// Default timeout for requests (ms)
    pub default_timeout_ms: u64,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
    /// Whether to log all requests
    pub audit_enabled: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            agent_id: uuid::Uuid::new_v4().to_string(),
            default_timeout_ms: 30_000,
            max_concurrent: 10,
            audit_enabled: true,
        }
    }
}

/// Main entry point for Elohim agent invocation.
///
/// Orchestrates LLM backends with constitutional reasoning.
pub struct ElohimAgentService {
    /// Configuration
    config: ServiceConfig,
    /// Available LLM backends
    backends: Vec<Arc<dyn LlmBackend>>,
    /// Constitutional stack
    stack: Arc<RwLock<Option<ConstitutionalStack>>>,
    /// Capability registry
    capabilities: Arc<CapabilityRegistry>,
    /// Audit log
    audit: Arc<AuditLog>,
    /// Whether service is initialized
    initialized: Arc<RwLock<bool>>,
}

impl ElohimAgentService {
    /// Create a new service with the given backends.
    pub fn new(backends: Vec<Arc<dyn LlmBackend>>) -> Self {
        Self {
            config: ServiceConfig::default(),
            backends,
            stack: Arc::new(RwLock::new(None)),
            capabilities: Arc::new(CapabilityRegistry::new()),
            audit: Arc::new(AuditLog::new()),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Create with configuration.
    pub fn with_config(mut self, config: ServiceConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.config.agent_id
    }

    /// Initialize the service with a constitutional stack.
    pub async fn initialize(&self, context: StackContext) -> Result<(), ServiceError> {
        info!(agent_id = %self.config.agent_id, "Initializing ElohimAgentService");

        // Build constitutional stack with defaults
        let stack = ConstitutionalStack::build_defaults(context);

        {
            let mut stack_guard = self.stack.write().await;
            *stack_guard = Some(stack);
        }

        {
            let mut init = self.initialized.write().await;
            *init = true;
        }

        info!("ElohimAgentService initialized");
        Ok(())
    }

    /// Register capabilities this service can handle.
    pub async fn register_capabilities(&self, capabilities: impl IntoIterator<Item = ElohimCapability>) {
        self.capabilities.register_all(capabilities).await;
    }

    /// Check if service is initialized.
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Invoke the agent with a request (non-streaming).
    pub async fn invoke(&self, request: ElohimRequest) -> Result<ElohimResponse, ServiceError> {
        // Check initialization
        if !self.is_initialized().await {
            return Err(ServiceError::NotInitialized);
        }

        // Log request
        let entry_id = if self.config.audit_enabled {
            Some(self.audit.log_request(&request).await)
        } else {
            None
        };

        debug!(
            request_id = %request.request_id,
            capability = ?request.capability,
            "Processing request"
        );

        // Check capability
        if !self.capabilities.is_available(request.capability).await {
            let response = ElohimResponse::declined(
                &request.request_id,
                &self.config.agent_id,
                format!("Capability {:?} not available", request.capability),
            );

            if let Some(entry_id) = entry_id {
                self.audit.log_response(&entry_id, &response).await;
            }

            return Ok(response);
        }

        // Get constitutional stack
        let stack = self.stack.read().await;
        let stack = stack.as_ref().ok_or(ServiceError::NotInitialized)?;

        // Select backend
        let backend = self.select_backend().await?;

        // Process the request
        let response = self
            .process_request(&request, stack, backend.as_ref())
            .await?;

        // Log response
        if let Some(entry_id) = entry_id {
            self.audit.log_response(&entry_id, &response).await;
        }

        Ok(response)
    }

    /// Invoke with streaming response.
    pub async fn invoke_stream(&self, request: ElohimRequest) -> Result<TokenStream, ServiceError> {
        // For now, fall back to non-streaming
        let response = self.invoke(request).await?;

        // Convert response content to stream
        let content = match &response.payload {
            ResponsePayload::Generic { data } => {
                serde_json::to_string_pretty(data).unwrap_or_default()
            }
            _ => serde_json::to_string_pretty(&response.payload).unwrap_or_default(),
        };

        Ok(TokenStream::from_complete(
            crate::backend::traits::CompletionResponse {
                content,
                finish_reason: crate::backend::traits::FinishReason::Stop,
                usage: crate::backend::traits::Usage {
                    prompt_tokens: response.cost.input_tokens,
                    completion_tokens: response.cost.output_tokens,
                },
            },
        ))
    }

    /// Get recent audit entries.
    pub async fn get_audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        self.audit.recent(limit).await
    }

    /// Get available capabilities.
    pub async fn available_capabilities(&self) -> Vec<ElohimCapability> {
        self.capabilities.available().await
    }

    /// Select the best available backend.
    async fn select_backend(&self) -> Result<Arc<dyn LlmBackend>, ServiceError> {
        for backend in &self.backends {
            if backend.is_available().await {
                return Ok(Arc::clone(backend));
            }
        }
        Err(ServiceError::NoBackendAvailable)
    }

    /// Process a request with constitutional reasoning.
    async fn process_request(
        &self,
        request: &ElohimRequest,
        stack: &ConstitutionalStack,
        backend: &dyn LlmBackend,
    ) -> Result<ElohimResponse, ServiceError> {
        let start = std::time::Instant::now();

        // Build constitutional prompt
        let system_prompt = PromptAssembler::build_system_prompt(stack);

        // Build capability-specific prompt
        let user_prompt = self.build_capability_prompt(request);

        // Create completion request
        let completion_request = crate::backend::traits::CompletionRequest::user(&user_prompt)
            .with_system(&system_prompt)
            .with_max_tokens(2048)
            .with_temperature(0.7)
            .with_json_output();

        // Call backend
        let completion = backend.complete(completion_request).await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Parse response and build payload
        let payload = self.parse_capability_response(request.capability, &completion.content);

        // Build constitutional reasoning
        let reasoning = ConstitutionalReasoning {
            primary_principle: "Capability fulfillment".to_string(),
            interpretation: format!(
                "Processed {:?} request following constitutional guidelines",
                request.capability
            ),
            values_weighed: vec![],
            confidence: 0.85,
            precedents: vec![],
            new_precedent: false,
            stack_hash: stack.stack_hash().to_string(),
            determining_layer: request
                .capability
                .required_layer()
                .unwrap_or(constitution::ConstitutionalLayer::Individual),
        };

        let cost = ComputationCost {
            input_tokens: completion.usage.prompt_tokens,
            output_tokens: completion.usage.completion_tokens,
            processing_time_ms: duration_ms,
            constitutional_checks: 1,
        };

        Ok(ElohimResponse::fulfilled(
            &request.request_id,
            &self.config.agent_id,
            reasoning,
            payload,
            cost,
        ))
    }

    /// Build a capability-specific prompt.
    fn build_capability_prompt(&self, request: &ElohimRequest) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!(
            "Execute capability: {:?}\n\n",
            request.capability
        ));
        prompt.push_str(&format!(
            "Description: {}\n\n",
            request.capability.description()
        ));

        if let Some(content) = &request.params.content {
            prompt.push_str(&format!("Content to analyze:\n{}\n\n", content));
        }

        if let Some(content_id) = &request.params.content_id {
            prompt.push_str(&format!("Content ID: {}\n\n", content_id));
        }

        if let Some(query) = &request.params.query {
            prompt.push_str(&format!("Query: {}\n\n", query));
        }

        prompt.push_str("Respond with a JSON object containing your analysis and recommendations.\n");

        prompt
    }

    /// Parse capability-specific response.
    fn parse_capability_response(
        &self,
        capability: ElohimCapability,
        content: &str,
    ) -> ResponsePayload {
        // Try to parse as JSON
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
            // Try to map to specific payload types
            match capability {
                ElohimCapability::ContentSafetyReview => {
                    if let (Some(safe), Some(recommendation)) = (
                        data.get("safe").and_then(|v| v.as_bool()),
                        data.get("recommendation").and_then(|v| v.as_str()),
                    ) {
                        return ResponsePayload::SafetyReview {
                            safe,
                            issues: vec![],
                            recommendation: recommendation.to_string(),
                        };
                    }
                }
                ElohimCapability::SpiralDetection => {
                    if let Some(detected) = data.get("detected").and_then(|v| v.as_bool()) {
                        return ResponsePayload::SpiralDetection {
                            detected,
                            severity: data
                                .get("severity")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            signals: vec![],
                            suggested_response: data
                                .get("suggested_response")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                        };
                    }
                }
                _ => {}
            }

            // Fall back to generic
            ResponsePayload::Generic { data }
        } else {
            // Wrap raw content
            ResponsePayload::Generic {
                data: serde_json::json!({ "raw_response": content }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockBackend;

    #[tokio::test]
    async fn test_service_initialization() {
        let backend = Arc::new(MockBackend::default().with_response(r#"{"safe": true, "recommendation": "Content is safe"}"#));
        let service = ElohimAgentService::new(vec![backend]);

        assert!(!service.is_initialized().await);

        service
            .initialize(StackContext::agent_only("test-agent"))
            .await
            .unwrap();

        assert!(service.is_initialized().await);
    }

    #[tokio::test]
    async fn test_service_invoke() {
        let backend = Arc::new(
            MockBackend::default()
                .with_response(r#"{"safe": true, "recommendation": "Content is safe"}"#),
        );
        let service = ElohimAgentService::new(vec![backend]);

        service
            .initialize(StackContext::agent_only("test-agent"))
            .await
            .unwrap();

        service
            .register_capabilities([ElohimCapability::ContentSafetyReview])
            .await;

        let request = ElohimRequest::new(ElohimCapability::ContentSafetyReview, "user-123");
        let response = service.invoke(request).await.unwrap();

        assert_eq!(response.status, ResponseStatus::Fulfilled);
    }

    #[tokio::test]
    async fn test_capability_not_available() {
        let backend = Arc::new(MockBackend::default());
        let service = ElohimAgentService::new(vec![backend]);

        service
            .initialize(StackContext::agent_only("test-agent"))
            .await
            .unwrap();

        // Don't register any capabilities
        let request = ElohimRequest::new(ElohimCapability::ContentSafetyReview, "user-123");
        let response = service.invoke(request).await.unwrap();

        assert_eq!(response.status, ResponseStatus::Declined);
    }
}
