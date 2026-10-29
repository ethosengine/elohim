//! Rule row for batched EPR head resolution.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P1");

/// Batch resolution carries the same row semantics as single resolution.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::P2pEprBatch,
    open: Rule::Permit,
    community: Rule::Authorize,
    bound: Rule::Authorize,
    private: Rule::Authorize,
    unknown: Rule::Refuse,
    min_proof: Proof::SelfAssertedBinding,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
