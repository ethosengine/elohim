//! Error types for the gate interface.
//!
//! `GateError` variants that can be expressed as pure data (no runtime deps).
//! `GateResult<T>` is the canonical result alias used by both gate-client and
//! gate-client-zome.

use thiserror::Error;

pub type GateResult<T> = Result<T, GateError>;

/// Errors that can occur during gate invocation.
///
/// All variants are expressible without runtime or network dependencies so that
/// this type compiles for `wasm32-unknown-unknown`.
#[derive(Debug, Error)]
pub enum GateError {
    /// A step type or feature is not yet implemented in this phase.
    #[error("not yet implemented: {0}")]
    NotYetImplemented(String),

    /// Context assembly failed — a required pull could not resolve.
    #[error("context assembly failed: {0}")]
    ContextAssembly(String),

    /// Manifest resolution failed — gate name did not resolve to a process CID.
    #[error("manifest resolution failed: {0}")]
    ManifestResolution(String),

    /// Manifest inspection rejected the DAG as unsafe or uninspectable.
    #[error("manifest inspection rejected: {0}")]
    InspectionRejected(String),

    /// DAG execution encountered an invalid step or edge.
    #[error("DAG execution error: {0}")]
    DagExecution(String),

    /// Wisdom invocation failed — elohim-agent-service unreachable or errored.
    #[error("wisdom invocation failed: {0}")]
    WisdomInvocation(String),

    /// Transport-level error (HTTP, IPC, serialization).
    #[error("transport error: {0}")]
    Transport(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
