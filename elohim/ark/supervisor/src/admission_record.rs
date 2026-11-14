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

/// Recordable admission evidence, never a grant. No admission API accepts this
/// type as input; deserializing a permit cannot authorize a candidate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdmissionRecord {
    /// Exact executable/variant CID, never a platform label.
    pub(super) executable: Cid,
    /// Public effect only: required clauses and reasons, never the host inventory.
    pub(super) verdict: Verdict,
    /// Nonempty only for a permit under the own-build rule; names EVERY waiver.
    pub(super) own_build_rule: Vec<WaivedClause>,
    /// This device's Produce CID, present only when the whole admission permits
    /// under the own-build rule. Never a host inventory or a bare boolean.
    pub(super) own_produce: Option<Cid>,
}

impl AdmissionRecord {
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
