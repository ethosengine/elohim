//! Transport configuration for how the gate-client reaches the wisdom engine.
//!
//! Per spec §6.1: the client handles deployment-mode abstraction transparent
//! to callers. In-process when elohim-agent-service is co-located; HTTP/gRPC
//! when remote. Phase 0 defines the config shape; Phase 1 wires the transports.

use serde::{Deserialize, Serialize};

use crate::phase::Phase;

// ─── WisdomTransport ──────────────────────────────────────────────────────────

/// How `WisdomInvokeExecutor` obtains wisdom.
///
/// # Phase observation rule
///
/// `Phase::ElohimActive` is **observed from the actual call outcome**, never
/// from a config flag.  The response phase field is authoritative:
///
/// | Transport        | Backend available? | Phase result       |
/// |------------------|--------------------|--------------------|
/// | `Mock`           | n/a (no call)      | `DevContext`       |
/// | `InProcess`      | No API key         | `DevContext`       |
/// | `InProcess`      | Key set, call OK   | `ElohimActive`     |
/// | `InProcess`      | Call failed/unparseable | `DevContext` |
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WisdomTransport {
    /// Return hardcoded Allow + `Phase::DevContext`. Used when no wisdom service
    /// is wired or when `check_blocking` is called in sync zome contexts.
    #[default]
    Mock,
    /// In-process call to `elohim_agent::wisdom::invoke_wisdom`.
    /// `Phase::ElohimActive` ONLY if the underlying LLM call actually succeeded;
    /// `elohim-agent-service`'s response phase is authoritative.
    InProcess,
}

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
    /// How the `WisdomInvokeExecutor` reaches the wisdom engine.
    ///
    /// Default: `WisdomTransport::Mock` — hardcoded Allow + `Phase::DevContext`.
    /// Set to `WisdomTransport::InProcess` to enable real LLM inference via
    /// `elohim_agent::wisdom::invoke_wisdom` (requires an API key env var).
    #[serde(default)]
    pub wisdom_transport: WisdomTransport,
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
            wisdom_transport: WisdomTransport::Mock,
            phase_override: Some(Phase::DevContext),
            inspection_cache_path: None,
            elohim_id: None,
            elohim_substance_cid: None,
        }
    }
}

impl GateClientConfig {
    /// Construct a `GateClientConfig` from environment variables.
    ///
    /// Reads:
    ///
    /// - `ELOHIM_AGENT_WISDOM_TRANSPORT` — `"in-process"` selects
    ///   [`WisdomTransport::InProcess`]; anything else (or absent) falls back
    ///   to [`WisdomTransport::Mock`].
    /// - `ELOHIM_ID` — optional; overrides `elohim_id`.  When absent, the
    ///   runner falls back to [`DEV_CONTEXT_ELOHIM_ID`].
    /// - `ELOHIM_SUBSTANCE_CID` — optional; overrides `elohim_substance_cid`.
    ///   When absent, the runner falls back to [`DEV_CONTEXT_SUBSTANCE_CID`].
    ///
    /// # Activation note
    ///
    /// Setting `ELOHIM_AGENT_WISDOM_TRANSPORT=in-process` does **not** flip the
    /// phase.  [`Phase::ElohimActive`] is observed from the actual wisdom-invoke
    /// outcome; if no `ANTHROPIC_API_KEY` is set in the environment, the service
    /// returns `phase: dev-context` regardless of this setting.  See
    /// `ACTIVATION.md` for the full operator runbook.
    ///
    /// # No `ELOHIM_ACTIVE` flag
    ///
    /// There is deliberately no env var that forces `Phase::ElohimActive`.
    /// Phase is a property of what **actually happened** during inference —
    /// it cannot be asserted by configuration.
    ///
    /// [`DEV_CONTEXT_ELOHIM_ID`]: crate::dag::attestation::DEV_CONTEXT_ELOHIM_ID
    /// [`DEV_CONTEXT_SUBSTANCE_CID`]: crate::dag::attestation::DEV_CONTEXT_SUBSTANCE_CID
    pub fn from_env() -> Self {
        let wisdom_transport = match std::env::var("ELOHIM_AGENT_WISDOM_TRANSPORT")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "in-process" => WisdomTransport::InProcess,
            _ => WisdomTransport::Mock,
        };

        let elohim_id = std::env::var("ELOHIM_ID")
            .ok()
            .filter(|v| !v.trim().is_empty());

        let elohim_substance_cid = std::env::var("ELOHIM_SUBSTANCE_CID")
            .ok()
            .filter(|v| !v.trim().is_empty());

        Self {
            transport: Transport::Mock,
            wisdom_transport,
            // phase_override must remain None — phase is observed from inference,
            // not set by configuration.  Default::default() sets Some(DevContext)
            // which would override the service response; we clear it here.
            phase_override: None,
            inspection_cache_path: None,
            elohim_id,
            elohim_substance_cid,
        }
    }
}
