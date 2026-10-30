//! Rule row for application bundles reached by content slug.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Enforced;

/// Slug-addressed app behavior; CID-addressed apps use the HTTP blob fold.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::HttpApp,
    open: Rule::Permit,
    community: Rule::RequireIdentityPresent,
    bound: Rule::Authorize,
    private: Rule::Authorize,
    unknown: Rule::Authorize,
    min_proof: Proof::HopAsserted,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::ServeResidual,
    status: STATUS,
};
