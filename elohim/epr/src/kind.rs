//! EprKind enum — the nine EPR kinds defined by the graph substrate spec (§4.2).
//!
//! Each kind declares its required coupling legs. A malformed EPR — one missing
//! a required leg — is rejected at the structural validator (§7 stage 3).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
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
        }
    }
}
