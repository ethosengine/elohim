//! Elohim Agent - AI Agent Orchestration
//!
//! Provides the core infrastructure for invoking AI agents with:
//! - Trait-based LLM backends (vLLM/OpenAI, llama.cpp)
//! - Capability-based invocation matching Angular ElohimAgentService
//! - Constitutional reasoning audit trails
//! - Streaming support for real-time responses
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         ElohimAgentService              │
//! │  (Main entry point for invocations)     │
//! └────────────────┬────────────────────────┘
//!                  │
//!      ┌───────────┴───────────┐
//!      ▼                       ▼
//! ┌─────────────┐       ┌─────────────┐
//! │ LlmBackend  │       │ Constitution│
//! │ (OpenAI/    │       │ Stack       │
//! │  LlamaCpp)  │       │             │
//! └─────────────┘       └─────────────┘
//! ```

pub mod audit;
pub mod backend;
pub mod capability;
pub mod request;
pub mod response;
pub mod service;
pub mod stream;
pub mod types;

// Re-export main types for convenience
pub use backend::traits::{CompletionRequest, CompletionResponse, LlmBackend, LlmError};
pub use capability::types::ElohimCapability;
pub use request::{ElohimRequest, RequestPriority};
pub use response::{ElohimResponse, ResponseStatus};
pub use service::{ElohimAgentService, ServiceError};
pub use types::*;
