//! Residual rule row for public Holochain Content entries.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this residual plane.
pub const STATUS: Status = Status::Residual("DNA");

/// The DNA exposes every Content entry publicly and storage has no hook.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::HolochainDht,
    open: Rule::Permit,
    community: Rule::Permit,
    bound: Rule::Permit,
    private: Rule::Permit,
    unknown: Rule::Permit,
    min_proof: Proof::None,
    no_record: NoRecord::Permit,
    records_unavailable: RecordsUnavailable::ServeResidual,
    status: STATUS,
};
