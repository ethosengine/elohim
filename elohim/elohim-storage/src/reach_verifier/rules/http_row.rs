//! Rule row for single-content HTTP reads.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Enforced;

/// Single-row HTTP reads identify community callers and authorize stricter tiers.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::HttpRow,
    open: Rule::Permit,
    community: Rule::RequireIdentityPresent,
    bound: Rule::Authorize,
    private: Rule::Authorize,
    unknown: Rule::Authorize,
    min_proof: Proof::HopAsserted,
    no_record: NoRecord::Permit,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
