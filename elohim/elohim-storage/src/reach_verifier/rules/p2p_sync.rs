//! Rule row for peer CRDT synchronization.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P5");

/// Sync broadcasts open and community rows and withholds narrower rows.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::P2pSync,
    open: Rule::Broadcast,
    community: Rule::Broadcast,
    bound: Rule::Refuse,
    private: Rule::Refuse,
    unknown: Rule::Refuse,
    min_proof: Proof::SelfAssertedBinding,
    no_record: NoRecord::Permit,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
