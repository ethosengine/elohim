//! Rule row for shard protocol egress.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P4");

/// Shard behavior, including the pre-existing private custody gate.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::P2pShard,
    open: Rule::Permit,
    community: Rule::Permit,
    bound: Rule::Permit,
    private: Rule::Custody,
    unknown: Rule::Permit,
    min_proof: Proof::SelfAssertedBinding,
    no_record: NoRecord::Permit,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
