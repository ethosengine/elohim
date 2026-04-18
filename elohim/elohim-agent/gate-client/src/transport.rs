//! Transport configuration for how the gate-client reaches the wisdom engine.
//!
//! Per spec §6.1: the client handles deployment-mode abstraction transparent
//! to callers. In-process when elohim-agent-service is co-located; HTTP/gRPC
//! when remote. Phase 0 defines the config shape; Phase 1 wires the transports.

use serde::{Deserialize, Serialize};

use crate::phase::Phase;

/// How the gate-client reaches the elohim-agent-service for wisdom invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Transport {
    /// Co-located elohim-agent-service, direct Rust call.
    InProcess,
    /// Remote elohim-agent-service over HTTP.
    Http { url: String },
    /// Remote elohim-agent-service over gRPC (future).
    Grpc { url: String },
    /// Phase 0 / rehearsal stub — wisdom-invoke returns mocked Allow.
    Mock,
}

impl Default for Transport {
    fn default() -> Self {
        Transport::Mock
    }
}

/// Runtime configuration for the gate-client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateClientConfig {
    pub transport: Transport,
    /// If set, forces the phase regardless of live-elohim availability.
    /// Used in tests to assert dev-context behavior.
    pub phase_override: Option<Phase>,
    /// Path to the local inspection cache. None = in-memory only.
    pub inspection_cache_path: Option<String>,
}

impl Default for GateClientConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Mock,
            phase_override: Some(Phase::DevContext),
            inspection_cache_path: None,
        }
    }
}
