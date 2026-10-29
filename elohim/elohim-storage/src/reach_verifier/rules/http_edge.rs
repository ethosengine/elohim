//! Rule row for HTTP relationship and graph reads.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Enforced;

/// HTTP edge behavior extracted from the row gate.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::HttpEdge,
    open: Rule::Permit,
    community: Rule::RequireIdentityPresent,
    bound: Rule::Authorize,
    private: Rule::Authorize,
    unknown: Rule::Authorize,
    min_proof: Proof::HopAsserted,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
