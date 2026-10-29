//! Rule row for single EPR head resolution.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P1");

/// Single EPR resolution authorizes scoped rows against the resolved agent.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::P2pEprResolve,
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
