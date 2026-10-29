//! Rule row for reconciliation provider eligibility.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P13");

/// Provider eligibility broadcasts only distribution-safe reaches.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::ProvideReconcile,
    open: Rule::Broadcast,
    community: Rule::Broadcast,
    bound: Rule::Refuse,
    private: Rule::Refuse,
    unknown: Rule::Refuse,
    min_proof: Proof::None,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::ServeResidual,
    status: STATUS,
};
