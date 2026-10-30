//! Rule row for EPR atom serving.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this plane.
pub const STATUS: Status = Status::Pending("P8");

/// Scoped atoms serve only to their signer; open atoms serve to every peer.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::P2pEprAtom,
    open: Rule::Permit,
    community: Rule::AuthorOnly,
    bound: Rule::AuthorOnly,
    private: Rule::AuthorOnly,
    unknown: Rule::Refuse,
    min_proof: Proof::SelfAssertedBinding,
    no_record: NoRecord::Refuse,
    records_unavailable: RecordsUnavailable::Refuse,
    status: STATUS,
};
