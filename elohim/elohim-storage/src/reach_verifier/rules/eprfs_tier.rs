//! Rule row for eprfs roots and standalone objects.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Enforced;

/// Eprfs behavior extracted from `RootReachGate`.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::EprfsTier,
    open: Rule::Permit,
    community: Rule::RequireIdentityPresent,
    bound: Rule::Authorize,
    private: Rule::Custody,
    unknown: Rule::Authorize,
    min_proof: Proof::SelfAssertedBinding,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
