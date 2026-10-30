//! Rule row for HTTP content listings and search results.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Enforced;

/// HTTP lists resolve community callers and authorize stricter tiers per row.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::HttpList,
    open: Rule::Permit,
    community: Rule::RequireResolvedHuman,
    bound: Rule::Authorize,
    private: Rule::Authorize,
    unknown: Rule::Authorize,
    min_proof: Proof::HopAsserted,
    no_record: NoRecord::Permit,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
