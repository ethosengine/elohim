//! EprKind enum — the nine EPR kinds defined by the graph substrate spec (§4.2).
//!
//! Each kind declares its required coupling legs. A malformed EPR — one missing
//! a required leg — is rejected at the structural validator (§7 stage 3).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum EprKind {
    Content,
    Agent,
    Manifest,
    Claim,
    Observation,
    EconomicEvent,
    Commitment,
    Attestation,
    Delegation,
    /// Phase 3.5 — Trust-Compute Gradient substrate.
    /// A signed signal about another EPR: squelch / correction / retraction /
    /// quarantine. Carries graduated standing impact. Wire type:
    /// `elohim_storage::p2p::feedback_signal::FeedbackSignal`.
    FeedbackSignal,
    /// Phase 3.5 — Trust-Compute Gradient tending subsystem.
    /// Peer-private discernment signal: the human tends the shape of their
    /// attention. TTL-bounded classification (values-forward / fatigue /
    /// scope-mismatch / safety) feeding the elohim's tender conversation and
    /// the collective's anonymous wisdom layer. Stays on the agent's source
    /// chain; never gossiped. `Visibility::Private` HDI entry lands at T5.
    /// Wire type: `elohim_storage::p2p::attention_tending::AttentionTending`.
    AttentionTending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum CouplingLeg {
    Knowledge,
    Value,
    Governance,
}

impl EprKind {
    /// Return the coupling legs that MUST be present for this kind.
    pub const fn required_coupling(self) -> &'static [CouplingLeg] {
        use CouplingLeg::{Governance, Knowledge, Value};
        match self {
            EprKind::Content => &[Knowledge, Value, Governance],
            EprKind::Agent => &[Governance],
            EprKind::Manifest => &[Governance],
            EprKind::Claim => &[Knowledge],
            EprKind::Observation => &[Knowledge],
            EprKind::EconomicEvent => &[Value],
            EprKind::Commitment => &[Value, Governance],
            EprKind::Attestation => &[Governance],
            EprKind::Delegation => &[Governance],
            // FeedbackSignal: governance leg required (signed attestation about
            // another EPR's standing impact; requires issuer accountability).
            EprKind::FeedbackSignal => &[Governance],
            // AttentionTending: no required coupling legs. This is a peer-private
            // discernment signal about the local agent's own attention — it concerns
            // no other EPR's standing impact and requires no accountability to external
            // peers. The Visibility::Private HDI entry (T5) enforces that it stays
            // on the source chain. Knowledge/Value/Governance legs are all optional
            // enrichment that tending lifecycle (T15) may add; none are structurally
            // required at the wire layer.
            EprKind::AttentionTending => &[],
        }
    }
}
