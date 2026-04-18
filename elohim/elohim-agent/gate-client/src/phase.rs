//! Phase marker for dev-context vs elohim-active execution.
//!
//! Every `GateDecision` and `GateDecisionAttestation` carries a Phase field so
//! rehearsal-era decisions can be cleanly distinguished from real protocol
//! state when reputation aggregation turns on.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Execution phase for gate decisions.
///
/// See spec §5.5 (Dev-context attestation marker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// Pre-elohim-activation: wisdom-invoke is mocked; decisions are legible
    /// but carry no reputation weight in the post-activation graph.
    DevContext,

    /// Post-elohim-activation: real elohim sign decisions with real wisdom;
    /// reputation accumulates from these attestations.
    ElohimActive,
}

impl Phase {
    pub fn is_dev_context(&self) -> bool {
        matches!(self, Phase::DevContext)
    }

    pub fn is_elohim_active(&self) -> bool {
        matches!(self, Phase::ElohimActive)
    }
}

impl Default for Phase {
    fn default() -> Self {
        Phase::DevContext
    }
}
