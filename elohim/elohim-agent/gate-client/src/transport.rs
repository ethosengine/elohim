//! Transport configuration for how the gate-client reaches the wisdom engine.
//!
//! Per spec §6.1: the client handles deployment-mode abstraction transparent
//! to callers. In-process when elohim-agent-service is co-located; HTTP/gRPC
//! when remote. Phase 0 defines the config shape; Phase 1 wires the transports.

use serde::{Deserialize, Serialize};

use crate::phase::Phase;

/// How the gate-client reaches the elohim-agent-service for wisdom invocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Transport {
    /// Co-located elohim-agent-service, direct Rust call.
    InProcess,
    /// Remote elohim-agent-service over HTTP.
    Http { url: String },
    /// Remote elohim-agent-service over gRPC (future).
    Grpc { url: String },
    /// Phase 0 / rehearsal stub — wisdom-invoke returns mocked Allow.
    #[default]
    Mock,
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
    /// AgentPubKey (base64-encoded) of the elohim making gate decisions.
    ///
    /// Phase 4 DevContext: None → falls back to
    /// [`DEV_CONTEXT_ELOHIM_ID`](crate::dag::attestation::DEV_CONTEXT_ELOHIM_ID).
    /// Phase 6+: inject the real elohim agent key here.
    pub elohim_id: Option<String>,
    /// CID of the elohim's substance declaration (model-weights + constitution
    /// + deployment-context).
    ///
    /// Phase 4 DevContext: None → falls back to
    /// [`DEV_CONTEXT_SUBSTANCE_CID`](crate::dag::attestation::DEV_CONTEXT_SUBSTANCE_CID).
    /// Phase 6+: inject the real substance CID here.
    pub elohim_substance_cid: Option<String>,
}

impl Default for GateClientConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Mock,
            phase_override: Some(Phase::DevContext),
            inspection_cache_path: None,
            elohim_id: None,
            elohim_substance_cid: None,
        }
    }
}
