//! LLM Backend abstraction layer.
//!
//! Provides a clean trait-based interface for different LLM inference backends:
//! - OpenAI-compatible (vLLM, Ollama, OpenAI, etc.)
//! - Local llama.cpp (optional feature)
//! - Mock backend for testing

pub mod anthropic;
pub mod mock;
pub mod openai;
pub mod traits;

#[cfg(feature = "llamacpp")]
pub mod llamacpp;

pub use anthropic::AnthropicBackend;
pub use mock::MockBackend;
pub use openai::OpenAiBackend;
pub use traits::{CompletionRequest, CompletionResponse, LlmBackend, LlmError, ModelCapabilities};
