//! Rule row for local CRDT projection admission.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P5");

/// Projection broadcasts open and community rows and withholds stricter tiers.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::ProjectionSync,
    open: Rule::Broadcast,
    community: Rule::Broadcast,
    bound: Rule::Refuse,
    private: Rule::Refuse,
    unknown: Rule::Refuse,
    min_proof: Proof::None,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
