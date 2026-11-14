//! The recordable effect of host admission. Admission itself (the host contract,
//! the observed inventory, the own-build rule) lives in `ark_supervisor::admission`;
//! this pure core only carries its outcome, so it names no host-contract type and
//! keeps that crate out of every workspace that depends on `ark-core`.

use cid::Cid;
use elohim_epr::verdict::{Decision, Verdict};
use serde::{Deserialize, Serialize};

/// One requirement clause waived under the own-build rule, in the contract's own
/// wire spelling but with no host-contract type: the required axis, the required
/// predicate as recorded, and the clause source. Never a host value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaivedClause {
    /// Required axis, e.g. `isa.x86`.
    pub axis: String,
    /// Required predicate exactly as the artifact's requirement records it.
    pub predicate: serde_json::Value,
    /// Clause source as recorded (`extracted` is the only waivable source).
    pub source: String,
    /// Recorded narrowing provenance, when the clause carried one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrowing: Option<serde_json::Value>,
}

/// Ephemeral compatibility effect, reconstructed from an artifact's requirements
/// and local observations. Safe for a caller to record as fulfilment evidence.
///
/// An effect is produced by admission (`ark_supervisor::admission::admit_artifact`),
/// never hand-built by a caller. It is bound to one executable: candidate
/// authentication refuses an effect computed for any other executable, so an
/// own-build permit cannot be carried to different bytes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityEffect {
    /// Exact executable/variant CID, never a platform label.
    executable: Cid,
    /// Public effect only: required clauses and reasons, never the host inventory.
    verdict: Verdict,
    /// Nonempty only for a permit under the own-build rule; names EVERY waiver.
    own_build_rule: Vec<WaivedClause>,
    /// This device's Produce CID, present only when the whole admission permits
    /// under the own-build rule. Never a host inventory or a bare boolean.
    own_produce: Option<Cid>,
}

impl CompatibilityEffect {
    /// Admission's constructor. Only host admission calls this; a caller that
    /// assembles an effect by hand has not admitted anything.
    pub fn from_admission(
        executable: Cid,
        verdict: Verdict,
        own_build_rule: Vec<WaivedClause>,
        own_produce: Option<Cid>,
    ) -> Self {
        Self {
            executable,
            verdict,
            own_build_rule,
            own_produce,
        }
    }
    /// The executable this effect was computed for.
    pub fn executable(&self) -> Cid {
        self.executable
    }
    /// The recordable verdict; its checks carry required clauses, never host values.
    pub fn verdict(&self) -> &Verdict {
        &self.verdict
    }
    /// Whether admission permitted (plainly or under the own-build rule).
    pub fn permits(&self) -> bool {
        self.verdict.decision == Decision::Permit
    }
    /// Every clause waived under the own-build rule; empty for a plain permit or a refusal.
    pub fn own_build_rule(&self) -> &[WaivedClause] {
        &self.own_build_rule
    }
    /// This device's Produce CID when the own-build rule was used.
    pub fn own_produce(&self) -> Option<Cid> {
        self.own_produce
    }
}
