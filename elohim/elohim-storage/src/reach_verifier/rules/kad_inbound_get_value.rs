//! Residual row for inbound Kademlia GET_VALUE serving.

use super::{NoRecord, RecordsUnavailable, Rule, RuleSet, Status};
use crate::reach_verifier::{EgressPlane, Proof};

/// Adoption state for this residual plane.
pub const STATUS: Status = Status::Residual("libp2p-kad-store");

/// The libp2p record store has no application-level reach hook.
pub const RULES: RuleSet = RuleSet {
    plane: EgressPlane::KadInboundGetValue,
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
