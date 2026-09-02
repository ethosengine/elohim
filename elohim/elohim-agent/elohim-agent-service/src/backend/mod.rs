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

// The `llamacpp` feature (Cargo.toml) has no backend module: `backend/llamacpp.rs`
// was never committed. The gated `pub mod llamacpp;` that stood here made every
// workspace-wide `cargo fmt --check` fail ("failed to resolve mod") because
// rustfmt resolves module files regardless of cfg — it blocked the elohim-epr
// pre-push gate on 2026-09-02. Add the module back together with its file.

pub use anthropic::AnthropicBackend;
pub use mock::MockBackend;
pub use openai::OpenAiBackend;
pub use traits::{CompletionRequest, CompletionResponse, LlmBackend, LlmError, ModelCapabilities};
